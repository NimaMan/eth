//! Processed-block disk cache store (on-disk replay store primitive).
//!
//! Algorithm / layout:
//!   * One file per block at `<root>/<network>/<block>.v<SCHEMA>.pblock.zst`.
//!     The filename is block-number based so a range read derives every path
//!     from `start..=end` without fetching headers first. The schema version is
//!     baked into the name so a payload-layout change routes to a fresh filename
//!     instead of mass-invalidating (see `CACHE_SCHEMA_VERSION`).
//!   * Write: `ProcessedBlock` -> `ProcessedBlockDiskCacheEntry`
//!     (key + header + per-tx `CompactProcessedTransaction`, with
//!     bincode-hostile fields — struct logs, v3/v4 swaps, signed auths —
//!     extracted to JSON byte blobs) -> bincode -> zstd(level) -> atomic
//!     temp-file + rename.
//!   * Read: zstd-decode -> bincode-decode -> validate key/header -> rebuild
//!     `ProcessedBlock`. A decode/validation failure is a swallowed cache miss.
//!   * Pruning is FIFO by block number to a retain target; coverage scans the
//!     directory and reports per-chain byte/block ranges.
//!
//! The `bench_*` items near the bottom expose candidate codecs (Full/Lean field
//! sets, BincodeJson/Msgpack) used by the format benchmark to compare space and
//! read time before a format change is committed.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    processed_block_trace_config_hash, processed_tx_provider::CompactProcessedTransaction,
    ProcessedBlock, ProcessedBlockTransactions,
};
use alloy_primitives::B256;
use eyre::{bail, Result, WrapErr};
use reth_chain_query::provider::BlockHeader;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::reader::ProcessedBlockDiskCacheReader;
use super::writer::ProcessedBlockDiskCacheWriter;

const TRACE_ENGINE_ID: &str = "fresh_inspector";
/// Bincode payload schema version, baked into the cache filename.
///
/// The cache payload (`CompactProcessedTransaction` / `ProcessedBlockDiskCacheEntry`)
/// is bincode-encoded, which is positional and NOT self-describing: adding or
/// reordering a field changes the byte layout, so an old payload either fails to
/// deserialize or silently mis-decodes. Without a version in the filename, such a
/// change makes EVERY existing `.pblock` entry a swallowed deserialize-failure
/// (silent cache miss), forcing a full fresh re-trace of the whole cache at once.
///
/// Bumping this version routes reads/writes to a NEW filename
/// (`<block>.v<N>.pblock.zst`), so old entries are never read against an
/// incompatible layout — the cache refreshes cleanly per block on demand instead
/// of mass-invalidating. BUMP THIS whenever the bincode layout of the cached
/// types changes (e.g. adding `internal_erc20_calls`), OR whenever a cached
/// DERIVED field is recomputed differently (e.g. `address_balance_changes` now
/// folds in event-less `internal_erc20_transfers`) so stale derivations refresh.
///
/// v3 combines two orthogonal payload changes that landed together:
///   * (dev) add persisted `internal_erc20_transfers` and fold those event-less
///     ERC-20 transfers into `address_balance_changes`.
///   * (cache-format-opt) shrink the payload — truncate raw calldata `input`
///     (the single largest field) to its 4-byte selector, and fully drop
///     `struct_logs`, `erc1155_contracts`, `access_list`,
///     `blob_versioned_hashes`. See `strip_unpersisted_entry_fields` and
///     block/README.md. NOTE: the strip MUST NOT drop `internal_erc20_transfers`.
const CACHE_SCHEMA_VERSION: u32 = 3;
const CACHE_FILE_SUFFIX: &str = ".pblock.zst";
/// zstd compression level for cache payloads. Level 9 (vs the original 3) is
/// ~8-10% smaller for a modest write cost (~tens of ms/block; the live path
/// writes one block per ~12s and backfill is parallel) and NO read penalty —
/// zstd decompression speed is independent of the level a frame was written at.
/// Combined with the v3 field drop this yields ~-22% vs the original v2/zstd-3.
const CACHE_ZSTD_LEVEL: i32 = 9;
const ETHEREUM_MAINNET_CHAIN_ID: u64 = 1;
const ETHEREUM_MAINNET_NETWORK: &str = "ethereum-mainnet";

#[derive(Debug, Clone)]
pub struct ProcessedBlockDiskCacheStore {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessedBlockDiskCacheKey {
    pub chain_id: u64,
    pub network: String,
    pub block_number: u64,
    pub block_hash: Option<B256>,
    pub trace_engine: String,
    pub trace_config_hash: B256,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedBlockDiskCacheCoverage {
    pub root: String,
    pub chain_count: usize,
    pub block_count: usize,
    pub file_count: usize,
    pub total_bytes: u64,
    pub average_bytes_per_block: Option<f64>,
    pub trace_engine: String,
    pub trace_config_hash: String,
    pub chains: Vec<ProcessedBlockDiskCacheChainCoverage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedBlockDiskCacheChainCoverage {
    pub chain_id: u64,
    pub network: String,
    pub path: String,
    pub block_count: usize,
    pub file_count: usize,
    pub total_bytes: u64,
    pub average_bytes_per_block: Option<f64>,
    pub min_block: Option<u64>,
    pub max_block: Option<u64>,
    pub ranges: Vec<ProcessedBlockDiskCacheBlockRange>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedBlockDiskCacheBlockRange {
    pub start_block: u64,
    pub end_block: u64,
    pub block_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedBlockDiskCacheEntry {
    key: ProcessedBlockDiskCacheKey,
    header: BlockHeader,
    transactions: Vec<ProcessedBlockDiskCacheTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessedBlockDiskCacheTransaction {
    processed: CompactProcessedTransaction,
    signed_authorizations_json: Option<Vec<Vec<u8>>>,
    struct_logs_json: Option<Vec<u8>>,
    uniswap_v3_swaps_json: Option<Vec<u8>>,
    uniswap_v4_modifies_json: Option<Vec<u8>>,
    uniswap_v4_swaps_json: Option<Vec<u8>>,
    uniswap_v4_balance_deltas_json: Option<Vec<u8>>,
    processing_error: Option<String>,
}

impl ProcessedBlockDiskCacheKey {
    pub fn new(chain_id: u64, header: &BlockHeader) -> Self {
        Self {
            chain_id,
            network: network_for_chain_id(chain_id),
            block_number: header.number,
            block_hash: Some(header.hash),
            trace_engine: TRACE_ENGINE_ID.to_string(),
            trace_config_hash: processed_block_trace_config_hash(true),
        }
    }

    pub fn for_block_number(chain_id: u64, block_number: u64) -> Self {
        Self {
            chain_id,
            network: network_for_chain_id(chain_id),
            block_number,
            block_hash: None,
            trace_engine: TRACE_ENGINE_ID.to_string(),
            trace_config_hash: processed_block_trace_config_hash(true),
        }
    }
}

impl ProcessedBlockDiskCacheStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn reader(&self) -> ProcessedBlockDiskCacheReader {
        ProcessedBlockDiskCacheReader::new(self.clone())
    }

    pub fn writer(&self, chain_id: u64) -> ProcessedBlockDiskCacheWriter {
        ProcessedBlockDiskCacheWriter::new(self.clone(), chain_id)
    }

    pub fn key_for_block(
        &self,
        chain_id: u64,
        block: &ProcessedBlock,
    ) -> ProcessedBlockDiskCacheKey {
        ProcessedBlockDiskCacheKey::new(chain_id, &block.header)
    }

    pub fn contains(&self, key: &ProcessedBlockDiskCacheKey) -> bool {
        self.path_for_key(key).exists()
    }

    pub fn cached_key_for_block_number(
        &self,
        chain_id: u64,
        block_number: u64,
    ) -> Result<Option<ProcessedBlockDiskCacheKey>> {
        let key = ProcessedBlockDiskCacheKey::for_block_number(chain_id, block_number);
        Ok(self.contains(&key).then_some(key))
    }

    pub fn get(&self, key: &ProcessedBlockDiskCacheKey) -> Result<Option<ProcessedBlock>> {
        let path = self.path_for_key(key);
        if !path.exists() {
            return Ok(None);
        }

        let bytes = read_cache_file(&path)?;
        let decoded = match zstd::stream::decode_all(bytes.as_slice()) {
            Ok(decoded) => decoded,
            Err(error) => {
                tracing::debug!(
                    path = %path.display(),
                    error = %error,
                    "processed block disk cache entry is not readable; treating as cache miss"
                );
                return Ok(None);
            }
        };
        let entry = match decode_cache_entry(&decoded) {
            Ok(entry) => entry,
            Err(error) => {
                tracing::debug!(
                    path = %path.display(),
                    error = %error,
                    "processed block disk cache entry is not decodable; treating as cache miss"
                );
                return Ok(None);
            }
        };
        if let Err(error) = validate_cache_entry(&entry, key) {
            tracing::debug!(
                path = %path.display(),
                error = %error,
                "processed block disk cache entry is stale or incompatible; treating as cache miss"
            );
            return Ok(None);
        }
        match entry.into_processed_block() {
            Ok(block) => Ok(Some(block)),
            Err(error) => {
                tracing::debug!(
                    path = %path.display(),
                    error = %error,
                    "processed block disk cache entry payload is invalid; treating as cache miss"
                );
                Ok(None)
            }
        }
    }

    pub fn put(&self, key: &ProcessedBlockDiskCacheKey, block: &ProcessedBlock) -> Result<()> {
        let path = self.path_for_key(key);
        let parent = path
            .parent()
            .ok_or_else(|| eyre::eyre!("cache path has no parent: {}", path.display()))?;
        fs::create_dir_all(parent)?;

        let mut entry = ProcessedBlockDiskCacheEntry::from_block(key.clone(), block)?;
        // v3 payload: do not persist fields no cache consumer reads (see
        // `strip_unpersisted_entry_fields`). This is the on-disk format; raw
        // calldata is recoverable from the reth DB if ever needed.
        strip_unpersisted_entry_fields(&mut entry);
        let bytes = encode_cache_entry(&entry)?;
        let bytes = zstd::stream::encode_all(bytes.as_slice(), CACHE_ZSTD_LEVEL)?;
        let temp_path = parent.join(format!(
            ".{}.tmp-{}-{}",
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("processed-block"),
            std::process::id(),
            monotonic_nanos()
        ));

        let write_result = (|| -> Result<()> {
            let mut file = fs::File::create(&temp_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp_path, &path)?;
            Ok(())
        })();

        if write_result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }

        write_result
    }

    pub fn prune_chain_to_recent_blocks(&self, chain_id: u64, retain_blocks: u64) -> Result<usize> {
        let retain_blocks = usize::try_from(retain_blocks).unwrap_or(usize::MAX);
        let chain_path = self.network_path(chain_id);
        let entries = match fs::read_dir(&chain_path) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(err) => return Err(err.into()),
        };

        let mut block_files = Vec::new();
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let Some(block_number) = entry
                .file_name()
                .to_str()
                .and_then(block_number_from_cache_file_name)
            else {
                continue;
            };
            block_files.push((block_number, entry.path()));
        }

        if block_files.len() <= retain_blocks {
            return Ok(0);
        }

        block_files.sort_unstable_by(|left, right| right.0.cmp(&left.0));
        let mut removed = 0;
        for (_, path) in block_files.iter().skip(retain_blocks) {
            match fs::remove_file(path) {
                Ok(()) => removed += 1,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(err.into()),
            }
        }
        Ok(removed)
    }

    pub fn remove_block(&self, chain_id: u64, block_number: u64) -> Result<()> {
        let key = ProcessedBlockDiskCacheKey::for_block_number(chain_id, block_number);
        let path = self.path_for_key(&key);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn coverage(&self) -> Result<ProcessedBlockDiskCacheCoverage> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ProcessedBlockDiskCacheCoverage {
                    root: self.root.display().to_string(),
                    chain_count: 0,
                    block_count: 0,
                    file_count: 0,
                    total_bytes: 0,
                    average_bytes_per_block: None,
                    trace_engine: TRACE_ENGINE_ID.to_string(),
                    trace_config_hash: format!("{:#x}", processed_block_trace_config_hash(true)),
                    chains: Vec::new(),
                });
            }
            Err(err) => return Err(err.into()),
        };

        let mut chains = Vec::new();
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let Some(network) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            let Some(chain_id) = chain_id_for_network(&network) else {
                continue;
            };
            let chain = self.coverage_for_chain(chain_id, &network, &entry.path())?;
            chains.push(chain);
        }

        chains.sort_by_key(|chain| chain.chain_id);
        let block_count = chains.iter().map(|chain| chain.block_count).sum();
        let file_count = chains.iter().map(|chain| chain.file_count).sum();
        let total_bytes = chains.iter().map(|chain| chain.total_bytes).sum();
        let average_bytes_per_block = average_bytes(total_bytes, block_count);

        Ok(ProcessedBlockDiskCacheCoverage {
            root: self.root.display().to_string(),
            chain_count: chains.len(),
            block_count,
            file_count,
            total_bytes,
            average_bytes_per_block,
            trace_engine: TRACE_ENGINE_ID.to_string(),
            trace_config_hash: format!("{:#x}", processed_block_trace_config_hash(true)),
            chains,
        })
    }

    fn coverage_for_chain(
        &self,
        chain_id: u64,
        network: &str,
        chain_path: &Path,
    ) -> Result<ProcessedBlockDiskCacheChainCoverage> {
        let entries = match fs::read_dir(chain_path) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ProcessedBlockDiskCacheChainCoverage {
                    chain_id,
                    network: network.to_string(),
                    path: chain_path.display().to_string(),
                    block_count: 0,
                    file_count: 0,
                    total_bytes: 0,
                    average_bytes_per_block: None,
                    min_block: None,
                    max_block: None,
                    ranges: Vec::new(),
                });
            }
            Err(err) => return Err(err.into()),
        };

        let mut block_numbers = Vec::new();
        let mut file_count = 0;
        let mut total_bytes = 0;

        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let Some(block_number) = entry
                .file_name()
                .to_str()
                .and_then(block_number_from_cache_file_name)
            else {
                continue;
            };

            block_numbers.push(block_number);
            file_count += 1;
            total_bytes += entry.metadata()?.len();
        }

        block_numbers.sort_unstable();
        let ranges = block_ranges(&block_numbers);
        let min_block = block_numbers.first().copied();
        let max_block = block_numbers.last().copied();
        let average_bytes_per_block = average_bytes(total_bytes, block_numbers.len());

        Ok(ProcessedBlockDiskCacheChainCoverage {
            chain_id,
            network: network.to_string(),
            path: chain_path.display().to_string(),
            block_count: block_numbers.len(),
            file_count,
            total_bytes,
            average_bytes_per_block,
            min_block,
            max_block,
            ranges,
        })
    }

    fn path_for_key(&self, key: &ProcessedBlockDiskCacheKey) -> PathBuf {
        self.network_path(key.chain_id)
            .join(cache_file_name(key.block_number))
    }

    fn network_path(&self, chain_id: u64) -> PathBuf {
        self.root.join(network_for_chain_id(chain_id))
    }
}

fn read_cache_file(path: &Path) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    advise_sequential(&file);

    let capacity = file
        .metadata()
        .ok()
        .and_then(|metadata| usize::try_from(metadata.len()).ok())
        .unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    file.read_to_end(&mut bytes)?;
    advise_dontneed(&file);
    Ok(bytes)
}

#[cfg(unix)]
fn advise_sequential(file: &fs::File) {
    use std::os::fd::AsRawFd;

    let _ = unsafe { libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_SEQUENTIAL) };
}

#[cfg(not(unix))]
fn advise_sequential(_file: &fs::File) {}

#[cfg(unix)]
fn advise_dontneed(file: &fs::File) {
    use std::os::fd::AsRawFd;

    let _ = unsafe { libc::posix_fadvise(file.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) };
}

#[cfg(not(unix))]
fn advise_dontneed(_file: &fs::File) {}

fn block_ranges(block_numbers: &[u64]) -> Vec<ProcessedBlockDiskCacheBlockRange> {
    let mut ranges = Vec::new();
    let Some(&first) = block_numbers.first() else {
        return ranges;
    };

    let mut start = first;
    let mut previous = first;
    for &block_number in block_numbers.iter().skip(1) {
        if block_number != previous + 1 {
            ranges.push(ProcessedBlockDiskCacheBlockRange {
                start_block: start,
                end_block: previous,
                block_count: previous - start + 1,
            });
            start = block_number;
        }
        previous = block_number;
    }

    ranges.push(ProcessedBlockDiskCacheBlockRange {
        start_block: start,
        end_block: previous,
        block_count: previous - start + 1,
    });
    ranges
}

impl ProcessedBlockDiskCacheEntry {
    fn from_block(key: ProcessedBlockDiskCacheKey, block: &ProcessedBlock) -> Result<Self> {
        Ok(Self {
            key,
            header: block.header.clone(),
            transactions: block
                .transactions
                .iter()
                .map(ProcessedBlockDiskCacheTransaction::from_block_transaction)
                .collect::<Result<Vec<_>>>()?,
        })
    }

    pub fn into_processed_block(self) -> Result<ProcessedBlock> {
        Ok(ProcessedBlock {
            header: self.header,
            transactions: self
                .transactions
                .into_iter()
                .map(ProcessedBlockDiskCacheTransaction::into_block_transaction)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

fn encode_cache_entry(entry: &ProcessedBlockDiskCacheEntry) -> Result<Vec<u8>> {
    bincode::serialize(entry).wrap_err("failed to encode processed block disk cache entry")
}

fn decode_cache_entry(bytes: &[u8]) -> Result<ProcessedBlockDiskCacheEntry> {
    bincode::deserialize(bytes).wrap_err("failed to decode processed block disk cache entry")
}

fn validate_cache_entry(
    entry: &ProcessedBlockDiskCacheEntry,
    expected: &ProcessedBlockDiskCacheKey,
) -> Result<()> {
    if entry.key.chain_id != expected.chain_id
        || entry.key.network != expected.network
        || entry.key.block_number != expected.block_number
        || entry.key.trace_engine != TRACE_ENGINE_ID
        || entry.key.trace_config_hash != processed_block_trace_config_hash(true)
    {
        bail!(
            "cache key mismatch: expected {:?}, found {:?}",
            expected,
            entry.key
        );
    }
    if entry.header.number != expected.block_number {
        bail!(
            "cache header block number mismatch: expected {}, found {}",
            expected.block_number,
            entry.header.number
        );
    }
    if let Some(expected_hash) = expected.block_hash {
        if entry.header.hash != expected_hash {
            bail!(
                "cache header hash mismatch: expected {:?}, found {:?}",
                expected_hash,
                entry.header.hash
            );
        }
    }
    if let Some(entry_hash) = entry.key.block_hash {
        if entry.header.hash != entry_hash {
            bail!(
                "cache key/header hash mismatch: key {:?}, header {:?}",
                entry_hash,
                entry.header.hash
            );
        }
    }
    Ok(())
}

impl ProcessedBlockDiskCacheTransaction {
    fn from_block_transaction(tx: &ProcessedBlockTransactions) -> Result<Self> {
        let mut processed = CompactProcessedTransaction::from_processed(&tx.processed);
        let signed_authorizations_json = processed
            .signed_authorizations
            .take()
            .map(|auths| {
                auths
                    .into_iter()
                    .map(|auth| serde_json::to_vec(&auth))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
            .wrap_err("failed to encode signed authorizations for processed block disk cache")?;
        let struct_logs_json = processed
            .struct_logs
            .take()
            .map(|logs| serde_json::to_vec(&logs))
            .transpose()
            .wrap_err("failed to encode struct logs for processed block disk cache")?;
        let uniswap_v3_swaps_json =
            take_json_vec(&mut processed.uniswap_v3_swaps, "uniswap v3 swaps")?;
        let uniswap_v4_modifies_json = take_json_vec(
            &mut processed.uniswap_v4_modifies,
            "uniswap v4 modify-liquidity events",
        )?;
        let uniswap_v4_swaps_json =
            take_json_vec(&mut processed.uniswap_v4_swaps, "uniswap v4 swaps")?;
        let uniswap_v4_balance_deltas_json = take_json_vec(
            &mut processed.uniswap_v4_balance_deltas,
            "uniswap v4 balance deltas",
        )?;
        let entry = Self {
            processed,
            signed_authorizations_json,
            struct_logs_json,
            uniswap_v3_swaps_json,
            uniswap_v4_modifies_json,
            uniswap_v4_swaps_json,
            uniswap_v4_balance_deltas_json,
            processing_error: tx.processing_error.clone(),
        };
        Ok(entry)
    }

    fn into_block_transaction(mut self) -> Result<ProcessedBlockTransactions> {
        self.processed.signed_authorizations = self
            .signed_authorizations_json
            .map(|items| {
                items
                    .into_iter()
                    .map(|bytes| serde_json::from_slice(&bytes))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
            .wrap_err("failed to decode signed authorizations from processed block disk cache")?;
        self.processed.struct_logs = self
            .struct_logs_json
            .map(|bytes| serde_json::from_slice(&bytes))
            .transpose()
            .wrap_err("failed to decode struct logs from processed block disk cache")?;
        restore_json_vec(
            &mut self.processed.uniswap_v3_swaps,
            self.uniswap_v3_swaps_json,
            "uniswap v3 swaps",
        )?;
        restore_json_vec(
            &mut self.processed.uniswap_v4_modifies,
            self.uniswap_v4_modifies_json,
            "uniswap v4 modify-liquidity events",
        )?;
        restore_json_vec(
            &mut self.processed.uniswap_v4_swaps,
            self.uniswap_v4_swaps_json,
            "uniswap v4 swaps",
        )?;
        restore_json_vec(
            &mut self.processed.uniswap_v4_balance_deltas,
            self.uniswap_v4_balance_deltas_json,
            "uniswap v4 balance deltas",
        )?;
        Ok(self.processed.into_block_transaction(self.processing_error))
    }
}

fn take_json_vec<T: Serialize>(value: &mut Option<Vec<T>>, label: &str) -> Result<Option<Vec<u8>>> {
    value
        .take()
        .map(|items| serde_json::to_vec(&items))
        .transpose()
        .wrap_err_with(|| format!("failed to encode {label} for processed block disk cache"))
}

fn restore_json_vec<T: DeserializeOwned>(
    target: &mut Option<Vec<T>>,
    bytes: Option<Vec<u8>>,
    label: &str,
) -> Result<()> {
    *target = bytes
        .map(|bytes| serde_json::from_slice(&bytes))
        .transpose()
        .wrap_err_with(|| format!("failed to decode {label} from processed block disk cache"))?;
    Ok(())
}

fn monotonic_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

/// Schema-versioned cache filename for a block (`<block>.v<N>.pblock.zst`).
/// Exposed so out-of-store readers (e.g. the `dump_pblock` dev tools) build the
/// SAME name as the store and never read a stale, layout-incompatible version.
pub fn cache_file_name(block_number: u64) -> String {
    format!("{block_number}.v{CACHE_SCHEMA_VERSION}{CACHE_FILE_SUFFIX}")
}

fn block_number_from_cache_file_name(name: &str) -> Option<u64> {
    // Only recognize files written by the current schema version. Entries from a
    // previous version (`<block>.pblock.zst` or `<block>.v1.pblock.zst`) are
    // orphaned: never read against an incompatible layout, and not counted for
    // coverage/pruning of the current version.
    name.strip_suffix(CACHE_FILE_SUFFIX)
        .and_then(|value| value.strip_suffix(&format!(".v{CACHE_SCHEMA_VERSION}")))
        .and_then(|value| value.parse::<u64>().ok())
}

fn network_for_chain_id(chain_id: u64) -> String {
    match chain_id {
        ETHEREUM_MAINNET_CHAIN_ID => ETHEREUM_MAINNET_NETWORK.to_string(),
        _ => format!("chain-{chain_id}"),
    }
}

fn chain_id_for_network(network: &str) -> Option<u64> {
    match network {
        ETHEREUM_MAINNET_NETWORK => Some(ETHEREUM_MAINNET_CHAIN_ID),
        _ => network
            .strip_prefix("chain-")
            .and_then(|value| value.parse::<u64>().ok()),
    }
}

fn average_bytes(total_bytes: u64, blocks: usize) -> Option<f64> {
    (blocks > 0).then(|| total_bytes as f64 / blocks as f64)
}

// ===========================================================================
// Cache-format benchmark codecs
//
// These items exist so the format benchmark
// (examples/blocks/cache/benchmark_cache_formats.rs) can compare candidate
// on-disk encodings against the production codec WITHOUT re-running EVM replay:
// it decodes existing `.v2.pblock.zst` files into `ProcessedBlock`, then
// re-encodes each block under a candidate and measures size + decode time.
//
// Two orthogonal knobs are exposed; compression (zstd level / trained
// dictionary) is applied by the caller so it can be swept independently:
//   * CacheFieldSet  - Full (current payload) vs Lean (drop never-read fields).
//   * CacheSerCodec  - BincodeJson (production) vs Msgpack (no JSON extraction).
//
// The Lean drop-list is the set of fields no ProcessedBlock consumer reads,
// confirmed by a repo-wide consumer audit (see block/README.md "Stored vs
// consumed fields"): per-opcode struct logs, ERC-1155 contracts, the EIP-2930
// access list, and blob versioned hashes. (struct_logs dominates the savings.)
// ===========================================================================

/// Which fields a benchmark candidate carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheFieldSet {
    /// Every field the current production payload carries.
    Full,
    /// Drop fields no consumer reads (see module note / block/README.md).
    Lean,
}

/// Candidate serialization codec. Compression is applied separately by the
/// caller so zstd level/dictionary can be swept independently of the codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheSerCodec {
    /// Production codec: bincode entry, with bincode-hostile fields
    /// (struct logs, v3/v4 swaps, signed auths) JSON-extracted first.
    BincodeJson,
    /// MessagePack over the whole compact entry. MessagePack is map-based and
    /// handles `serde(flatten)`/maps natively, so no JSON extraction is needed.
    Msgpack,
}

/// Number of leading calldata bytes (the 4-byte function selector) the v3 cache
/// keeps. Consumers that still read cached `input` only do threshold checks
/// (`len >= 4` for route classification, `!is_empty()` for a direct-call
/// trigger); truncating to the selector preserves both exactly
/// (`min(len,4) >= 4 ⟺ len >= 4`, and `empty ⟺ empty`) for ~4 bytes/tx.
const CACHE_INPUT_SELECTOR_BYTES: usize = 4;

/// Strip fields the v3 cache payload does not persist, in place.
///
/// Fully dropped (a repo-wide consumer audit confirmed nothing reads them off a
/// cached tx): `struct_logs` (absent on the live path anyway), `erc1155_contracts`,
/// `access_list`, `blob_versioned_hashes`.
///
/// `input` (raw calldata, the single largest field) is truncated to its 4-byte
/// selector. The transferFrom (from,to,amount) eth_token used to decode from
/// calldata now comes from the trace-derived `internal_erc20_calls` (depth 0),
/// which is independent of `input` and kept; full calldata is recoverable from
/// the reth DB if ever needed.
///
/// NOTE: fields that LOOK droppable but ARE read must stay — `erc1155_transfers`
/// (tx_fund_flow), and `uniswap_v4_protocol_fee_updates` /
/// `uniswap_v4_dynamic_lp_fee_updates` /
/// `uniswap_v4_protocol_fee_controller_updates` (eth_token).
fn strip_unpersisted_compact_fields(processed: &mut CompactProcessedTransaction) {
    processed.struct_logs = None;
    processed.erc1155_contracts = None;
    processed.access_list = None;
    processed.blob_versioned_hashes = None;
    if let Some(input) = processed.input.as_mut() {
        input.truncate(CACHE_INPUT_SELECTOR_BYTES);
    }
}

/// Apply [`strip_unpersisted_compact_fields`] to an assembled cache entry,
/// including the separately-JSON-extracted struct logs.
fn strip_unpersisted_entry_fields(entry: &mut ProcessedBlockDiskCacheEntry) {
    for tx in &mut entry.transactions {
        tx.struct_logs_json = None;
        strip_unpersisted_compact_fields(&mut tx.processed);
    }
}

/// MessagePack benchmark entry: carries the compact transactions whole (no JSON
/// extraction). Mirrors `ProcessedBlockDiskCacheEntry` minus the validation key,
/// which is a fixed-size constant overhead irrelevant to the size comparison.
#[derive(Serialize, Deserialize)]
struct BenchMsgpackEntry {
    header: BlockHeader,
    transactions: Vec<BenchMsgpackTx>,
}

#[derive(Serialize, Deserialize)]
struct BenchMsgpackTx {
    processed: CompactProcessedTransaction,
    processing_error: Option<String>,
}

impl BenchMsgpackEntry {
    fn from_block(block: &ProcessedBlock, field_set: CacheFieldSet) -> Self {
        let transactions = block
            .transactions
            .iter()
            .map(|tx| {
                let mut processed = CompactProcessedTransaction::from_processed(&tx.processed);
                if field_set == CacheFieldSet::Lean {
                    strip_unpersisted_compact_fields(&mut processed);
                }
                BenchMsgpackTx {
                    processed,
                    processing_error: tx.processing_error.clone(),
                }
            })
            .collect();
        Self {
            header: block.header.clone(),
            transactions,
        }
    }

    fn into_processed_block(self) -> ProcessedBlock {
        ProcessedBlock {
            header: self.header,
            transactions: self
                .transactions
                .into_iter()
                .map(|tx| tx.processed.into_block_transaction(tx.processing_error))
                .collect(),
        }
    }
}

/// Serialize a block to UNCOMPRESSED payload bytes under a candidate field set
/// and codec. The caller applies compression (plain zstd at some level, or a
/// trained dictionary) so it can be benchmarked independently.
pub fn bench_serialize_block(
    key: &ProcessedBlockDiskCacheKey,
    block: &ProcessedBlock,
    field_set: CacheFieldSet,
    codec: CacheSerCodec,
) -> Result<Vec<u8>> {
    match codec {
        CacheSerCodec::BincodeJson => {
            let mut entry = ProcessedBlockDiskCacheEntry::from_block(key.clone(), block)?;
            if field_set == CacheFieldSet::Lean {
                strip_unpersisted_entry_fields(&mut entry);
            }
            encode_cache_entry(&entry)
        }
        CacheSerCodec::Msgpack => {
            let entry = BenchMsgpackEntry::from_block(block, field_set);
            rmp_serde::to_vec(&entry).wrap_err("failed to msgpack-encode benchmark cache entry")
        }
    }
}

/// Inverse of [`bench_serialize_block`] (post-decompression). Reconstructs a
/// `ProcessedBlock` so the benchmark can verify round-trip correctness.
pub fn bench_deserialize_block(bytes: &[u8], codec: CacheSerCodec) -> Result<ProcessedBlock> {
    match codec {
        CacheSerCodec::BincodeJson => decode_cache_entry(bytes)?.into_processed_block(),
        CacheSerCodec::Msgpack => {
            let entry: BenchMsgpackEntry = rmp_serde::from_slice(bytes)
                .wrap_err("failed to msgpack-decode benchmark cache entry")?;
            Ok(entry.into_processed_block())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, U256};
    use serde_json::json;
    use std::collections::HashMap;

    use crate::tx_processor::data_models::{
        ProcessedAccessListItem, TransactionFees, UniswapV3PoolCreatedEvent,
    };
    use crate::ProcessedTransaction;

    #[test]
    fn round_trips_replay_fields() {
        let root = std::env::temp_dir().join(format!(
            "processed-block-disk-cache-test-{}-{}",
            std::process::id(),
            monotonic_nanos()
        ));
        let store = ProcessedBlockDiskCacheStore::open(&root).expect("open cache");

        let mut tx = ProcessedTransaction::new(
            B256::repeat_byte(0x11),
            42,
            1_700_000_000,
            7,
            Address::repeat_byte(0x22),
            Some(Address::repeat_byte(0x33)),
            U256::from(123),
            true,
            9,
            2,
            vec![0xde, 0xad, 0xbe, 0xef, 0x11, 0x22, 0x33, 0x44],
        );
        tx.fees = TransactionFees::new_eip1559(
            U256::from(10),
            21_000,
            123_456,
            U256::from(20),
            U256::from(2),
        );
        tx.access_list.push(ProcessedAccessListItem {
            address: Address::repeat_byte(0x44),
            storage_keys: vec![B256::repeat_byte(0x55)],
        });
        tx.blob_versioned_hashes.push(B256::repeat_byte(0x66));
        tx.tx_type = "swap".to_string();
        tx.actions.push("token_tracking".to_string());
        tx.erc721_contracts.insert(Address::repeat_byte(0x77));
        tx.uniswap_v3_pools.push(UniswapV3PoolCreatedEvent {
            factory_address: Address::repeat_byte(0x99),
            token0: Address::repeat_byte(0x9a),
            token1: Address::repeat_byte(0x9b),
            fee: 3_000,
            tick_spacing: 60,
            pool: Address::repeat_byte(0x9c),
            log_index: 8,
        });
        tx.other_events.push(HashMap::from([(
            "debug".to_string(),
            json!({"kind": "state", "values": [1, 2, 3]}),
        )]));
        tx.latest_states.insert(
            Address::repeat_byte(0x88),
            json!({"reserve0": "1", "nested": {"ok": true}}),
        );

        let block = ProcessedBlock {
            header: BlockHeader {
                number: 42,
                hash: B256::repeat_byte(0xaa),
                parent_hash: B256::repeat_byte(0xbb),
                timestamp: 1_700_000_000,
                gas_limit: 30_000_000,
                gas_used: 1_000_000,
                base_fee_per_gas: Some(1),
                withdrawals_root: None,
                blob_gas_used: None,
                excess_blob_gas: None,
                parent_beacon_block_root: None,
                requests_hash: None,
                block_access_list_hash: None,
                slot_number: None,
            },
            transactions: vec![
                CompactProcessedTransaction::from_processed(&tx).into_block_transaction(None)
            ],
        };

        let write = store
            .writer(1)
            .write_processed_block(&block)
            .expect("write block");
        assert_eq!(write.key.network, "ethereum-mainnet");
        assert_eq!(write.key.block_hash, Some(B256::repeat_byte(0xaa)));
        assert!(root
            .join("ethereum-mainnet")
            .join(format!("42.v{CACHE_SCHEMA_VERSION}.pblock.zst"))
            .exists());

        let cached = store
            .get(&write.key)
            .expect("read cache")
            .expect("cached block");
        let cached_tx = &cached.transactions[0].processed;

        assert_eq!(cached_tx.fees.gas_limit, 123_456);
        assert_eq!(cached_tx.fees.max_fee_per_gas, Some(U256::from(20)));
        // v3 keeps only the 4-byte selector of calldata and fully drops the
        // other never-read fields. See strip_unpersisted_entry_fields.
        assert_eq!(
            cached_tx.input,
            vec![0xde, 0xad, 0xbe, 0xef],
            "v3 keeps only the 4-byte calldata selector"
        );
        assert!(cached_tx.access_list.is_empty(), "v3 drops access_list");
        assert!(
            cached_tx.blob_versioned_hashes.is_empty(),
            "v3 drops blob_versioned_hashes"
        );
        assert_eq!(cached_tx.tx_type, "swap");
        assert_eq!(cached_tx.actions, vec!["token_tracking"]);
        assert!(cached_tx
            .erc721_contracts
            .contains(&Address::repeat_byte(0x77)));
        assert_eq!(cached_tx.uniswap_v3_pools.len(), 1);
        assert_eq!(
            cached_tx.uniswap_v3_pools[0].factory_address,
            Address::repeat_byte(0x99)
        );
        assert_eq!(cached_tx.other_events[0]["debug"]["kind"], "state");
        assert_eq!(
            cached_tx.latest_states[&Address::repeat_byte(0x88)]["nested"]["ok"],
            true
        );

        let planned_key = ProcessedBlockDiskCacheKey::for_block_number(1, 42);
        let cached_by_block_number = store
            .get(&planned_key)
            .expect("read cache by block number")
            .expect("cached block by block number");
        assert_eq!(cached_by_block_number.header.hash, B256::repeat_byte(0xaa));

        let coverage = store.coverage().expect("coverage");
        assert_eq!(coverage.chain_count, 1);
        assert_eq!(coverage.block_count, 1);
        assert_eq!(coverage.file_count, 1);
        assert_eq!(coverage.chains[0].network, "ethereum-mainnet");
        assert_eq!(coverage.chains[0].ranges[0].start_block, 42);
        assert_eq!(coverage.chains[0].ranges[0].end_block, 42);

        std::fs::remove_dir_all(root).expect("remove temp cache");
    }

    #[test]
    fn stale_trace_hash_is_treated_as_miss() {
        let root = std::env::temp_dir().join(format!(
            "processed-block-disk-cache-stale-test-{}-{}",
            std::process::id(),
            monotonic_nanos()
        ));
        let store = ProcessedBlockDiskCacheStore::open(&root).expect("open cache");

        let block = ProcessedBlock {
            header: BlockHeader {
                number: 42,
                hash: B256::repeat_byte(0xaa),
                parent_hash: B256::repeat_byte(0xbb),
                timestamp: 1_700_000_000,
                gas_limit: 30_000_000,
                gas_used: 1_000_000,
                base_fee_per_gas: Some(1),
                withdrawals_root: None,
                blob_gas_used: None,
                excess_blob_gas: None,
                parent_beacon_block_root: None,
                requests_hash: None,
                block_access_list_hash: None,
                slot_number: None,
            },
            transactions: Vec::new(),
        };

        let key = store.key_for_block(1, &block);
        let path = store.path_for_key(&key);
        std::fs::create_dir_all(path.parent().expect("cache parent")).expect("create parent");
        let mut entry =
            ProcessedBlockDiskCacheEntry::from_block(key.clone(), &block).expect("cache entry");
        entry.key.trace_config_hash = B256::repeat_byte(0x99);
        let encoded = encode_cache_entry(&entry).expect("encode stale entry");
        let encoded =
            zstd::stream::encode_all(encoded.as_slice(), CACHE_ZSTD_LEVEL).expect("compress");
        std::fs::write(&path, encoded).expect("write stale cache entry");

        assert!(store.get(&key).expect("read stale cache").is_none());

        std::fs::remove_dir_all(root).expect("remove temp cache");
    }
}
