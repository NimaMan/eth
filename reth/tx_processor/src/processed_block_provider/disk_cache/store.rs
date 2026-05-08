use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    processed_block_provider::CompactProcessedTransaction, processed_block_trace_config_hash,
    ProcessedBlock, ProcessedBlockTransactions,
};
use alloy_primitives::B256;
use eyre::{bail, Result};
use reth_chain_query::provider::BlockHeader;
use serde::{Deserialize, Serialize};

use super::reader::ProcessedBlockDiskCacheReader;
use super::writer::ProcessedBlockDiskCacheWriter;

const TRACE_ENGINE_ID: &str = "fresh_inspector";

#[derive(Debug, Clone)]
pub struct ProcessedBlockDiskCacheStore {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessedBlockDiskCacheKey {
    pub chain_id: u64,
    pub block_number: u64,
    pub block_hash: B256,
    pub trace_engine: String,
    pub trace_config_hash: B256,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedBlockDiskCacheCoverage {
    pub root: String,
    pub chain_count: usize,
    pub block_dir_count: usize,
    pub file_count: usize,
    pub total_bytes: u64,
    pub trace_engine: String,
    pub trace_config_hash: String,
    pub chains: Vec<ProcessedBlockDiskCacheChainCoverage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedBlockDiskCacheChainCoverage {
    pub chain_id: u64,
    pub path: String,
    pub block_dir_count: usize,
    pub file_count: usize,
    pub total_bytes: u64,
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
struct ProcessedBlockDiskCacheEntry {
    key: ProcessedBlockDiskCacheKey,
    header: BlockHeader,
    transactions: Vec<ProcessedBlockDiskCacheTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyProcessedBlockDiskCacheEntry {
    key: LegacyProcessedBlockDiskCacheKey,
    header: BlockHeader,
    transactions: Vec<ProcessedBlockDiskCacheTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyProcessedBlockDiskCacheKey {
    chain_id: u64,
    block_number: u64,
    block_hash: B256,
    _unused_a: u32,
    _unused_b: u32,
    trace_engine: String,
    trace_config_hash: B256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessedBlockDiskCacheTransaction {
    processed: CompactProcessedTransaction,
    processing_error: Option<String>,
}

impl ProcessedBlockDiskCacheKey {
    pub fn new(chain_id: u64, header: &BlockHeader) -> Self {
        Self {
            chain_id,
            block_number: header.number,
            block_hash: header.hash,
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
        let dir = self.current_block_dir(chain_id, block_number);
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };

        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("zst") {
                continue;
            }

            let bytes = fs::read(&path)?;
            let decoded = zstd::stream::decode_all(bytes.as_slice())?;
            let entry = decode_cache_entry(&decoded)?;
            if entry.key.chain_id != chain_id
                || entry.key.block_number != block_number
                || entry.key.trace_engine != TRACE_ENGINE_ID
                || entry.key.trace_config_hash != processed_block_trace_config_hash(true)
            {
                eyre::bail!(
                    "processed block disk cache key mismatch for {}: found {:?}",
                    path.display(),
                    entry.key
                );
            }
            return Ok(Some(entry.key));
        }

        Ok(None)
    }

    pub fn get(&self, key: &ProcessedBlockDiskCacheKey) -> Result<Option<ProcessedBlock>> {
        let path = self.path_for_key(key);
        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&path)?;
        let decoded = zstd::stream::decode_all(bytes.as_slice())?;
        let entry = decode_cache_entry(&decoded)?;
        if entry.key != *key {
            bail!(
                "processed block disk cache key mismatch for {}: expected {:?}, found {:?}",
                path.display(),
                key,
                entry.key
            );
        }
        Ok(Some(entry.into_processed_block()))
    }

    pub fn put(&self, key: &ProcessedBlockDiskCacheKey, block: &ProcessedBlock) -> Result<()> {
        let path = self.path_for_key(key);
        let parent = path
            .parent()
            .ok_or_else(|| eyre::eyre!("cache path has no parent: {}", path.display()))?;
        fs::create_dir_all(parent)?;

        let bytes = bincode::serialize(&ProcessedBlockDiskCacheEntry::from_block(
            key.clone(),
            block,
        ))?;
        let bytes = zstd::stream::encode_all(bytes.as_slice(), 1)?;
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
        let chain_path = self.root.join(format!("token-chain-{chain_id}"));
        let entries = match fs::read_dir(&chain_path) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(err) => return Err(err.into()),
        };

        let mut block_numbers = Vec::new();
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let Some(block_number) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix("block-"))
                .and_then(|value| value.parse::<u64>().ok())
            else {
                continue;
            };
            block_numbers.push(block_number);
        }

        if block_numbers.len() <= retain_blocks {
            return Ok(0);
        }

        block_numbers.sort_unstable_by(|left, right| right.cmp(left));
        let mut removed = 0;
        for block_number in block_numbers.iter().skip(retain_blocks) {
            match fs::remove_dir_all(chain_path.join(format!("block-{block_number}"))) {
                Ok(()) => removed += 1,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(err.into()),
            }
        }
        Ok(removed)
    }

    pub fn coverage(&self) -> Result<ProcessedBlockDiskCacheCoverage> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ProcessedBlockDiskCacheCoverage {
                    root: self.root.display().to_string(),
                    chain_count: 0,
                    block_dir_count: 0,
                    file_count: 0,
                    total_bytes: 0,
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
            let Some(chain_id) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix("token-chain-"))
                .and_then(|value| value.parse::<u64>().ok())
            else {
                continue;
            };
            chains.push(self.coverage_for_chain(chain_id, &entry.path())?);
        }

        chains.sort_by_key(|chain| chain.chain_id);
        let block_dir_count = chains.iter().map(|chain| chain.block_dir_count).sum();
        let file_count = chains.iter().map(|chain| chain.file_count).sum();
        let total_bytes = chains.iter().map(|chain| chain.total_bytes).sum();

        Ok(ProcessedBlockDiskCacheCoverage {
            root: self.root.display().to_string(),
            chain_count: chains.len(),
            block_dir_count,
            file_count,
            total_bytes,
            trace_engine: TRACE_ENGINE_ID.to_string(),
            trace_config_hash: format!("{:#x}", processed_block_trace_config_hash(true)),
            chains,
        })
    }

    fn coverage_for_chain(
        &self,
        chain_id: u64,
        chain_path: &Path,
    ) -> Result<ProcessedBlockDiskCacheChainCoverage> {
        let entries = match fs::read_dir(chain_path) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ProcessedBlockDiskCacheChainCoverage {
                    chain_id,
                    path: chain_path.display().to_string(),
                    block_dir_count: 0,
                    file_count: 0,
                    total_bytes: 0,
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
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let Some(block_number) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix("block-"))
                .and_then(|value| value.parse::<u64>().ok())
            else {
                continue;
            };

            let current_dir = self.current_block_dir(chain_id, block_number);
            let (block_files, block_bytes) = current_cache_file_stats(&current_dir)?;
            if block_files == 0 {
                continue;
            }
            block_numbers.push(block_number);
            file_count += block_files;
            total_bytes += block_bytes;
        }

        block_numbers.sort_unstable();
        let ranges = block_ranges(&block_numbers);
        let min_block = block_numbers.first().copied();
        let max_block = block_numbers.last().copied();

        Ok(ProcessedBlockDiskCacheChainCoverage {
            chain_id,
            path: chain_path.display().to_string(),
            block_dir_count: block_numbers.len(),
            file_count,
            total_bytes,
            min_block,
            max_block,
            ranges,
        })
    }

    fn path_for_key(&self, key: &ProcessedBlockDiskCacheKey) -> PathBuf {
        self.current_block_dir(key.chain_id, key.block_number)
            .join(format!("{:#x}.bin.zst", key.block_hash))
    }

    fn current_block_dir(&self, chain_id: u64, block_number: u64) -> PathBuf {
        self.root
            .join(format!("token-chain-{chain_id}"))
            .join(format!("block-{block_number}"))
    }
}

fn current_cache_file_stats(path: &Path) -> Result<(usize, u64)> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok((0, 0)),
        Err(err) => return Err(err.into()),
    };

    let mut file_count = 0;
    let mut total_bytes = 0;
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("zst") {
            continue;
        }
        file_count += 1;
        total_bytes += entry.metadata()?.len();
    }
    Ok((file_count, total_bytes))
}

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
    fn from_block(key: ProcessedBlockDiskCacheKey, block: &ProcessedBlock) -> Self {
        Self {
            key,
            header: block.header.clone(),
            transactions: block
                .transactions
                .iter()
                .map(|tx| ProcessedBlockDiskCacheTransaction {
                    processed: CompactProcessedTransaction::from_processed(&tx.processed),
                    processing_error: tx.processing_error.clone(),
                })
                .collect(),
        }
    }

    fn into_processed_block(self) -> ProcessedBlock {
        ProcessedBlock {
            header: self.header,
            transactions: self
                .transactions
                .into_iter()
                .map(ProcessedBlockDiskCacheTransaction::into_block_transaction)
                .collect(),
        }
    }
}

impl LegacyProcessedBlockDiskCacheEntry {
    fn into_current(self) -> ProcessedBlockDiskCacheEntry {
        ProcessedBlockDiskCacheEntry {
            key: self.key.into_current(),
            header: self.header,
            transactions: self.transactions,
        }
    }
}

impl LegacyProcessedBlockDiskCacheKey {
    fn into_current(self) -> ProcessedBlockDiskCacheKey {
        ProcessedBlockDiskCacheKey {
            chain_id: self.chain_id,
            block_number: self.block_number,
            block_hash: self.block_hash,
            trace_engine: self.trace_engine,
            trace_config_hash: self.trace_config_hash,
        }
    }
}

fn decode_cache_entry(bytes: &[u8]) -> Result<ProcessedBlockDiskCacheEntry> {
    match bincode::deserialize::<ProcessedBlockDiskCacheEntry>(bytes) {
        Ok(entry) => Ok(entry),
        Err(current_error) => {
            let legacy_entry: LegacyProcessedBlockDiskCacheEntry =
                bincode::deserialize(bytes).map_err(|legacy_error| {
                    eyre::eyre!(
                        "failed to decode processed block disk cache entry; current decode error: {current_error}; legacy decode error: {legacy_error}"
                    )
                })?;
            Ok(legacy_entry.into_current())
        }
    }
}

impl ProcessedBlockDiskCacheTransaction {
    fn into_block_transaction(self) -> ProcessedBlockTransactions {
        self.processed.into_block_transaction(self.processing_error)
    }
}

fn monotonic_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, U256};

    use crate::tx_processor::data_models::{ProcessedAccessListItem, TransactionFees};
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
            vec![0xde, 0xad, 0xbe, 0xef],
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
        let cached = store
            .get(&write.key)
            .expect("read cache")
            .expect("cached block");
        let cached_tx = &cached.transactions[0].processed;

        assert_eq!(cached_tx.fees.gas_limit, 123_456);
        assert_eq!(cached_tx.fees.max_fee_per_gas, Some(U256::from(20)));
        assert_eq!(cached_tx.input, vec![0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(cached_tx.access_list.len(), 1);
        assert_eq!(
            cached_tx.blob_versioned_hashes,
            vec![B256::repeat_byte(0x66)]
        );

        std::fs::remove_dir_all(root).expect("remove temp cache");
    }
}
