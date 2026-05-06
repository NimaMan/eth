use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::tx_processor::data_models::{
    AccessControlRoleGrantedEvent, AccessControlRoleRevokedEvent, ContractCreationEvent,
    ERC20ApprovalEvent, ERC20TransferEvent, InternalTransaction, OwnershipTransferStartedEvent,
    OwnershipTransferredEvent, ProcessedAccessListItem, ProxyAdminChangedEvent,
    TradingDisabledEvent, TradingEnabledEvent, TransactionFees, UniswapV2BurnEvent,
    UniswapV2MintEvent, UniswapV2PairCreatedEvent, UniswapV2SwapEvent, UniswapV2SyncEvent,
};
use crate::{
    processed_block_trace_config_hash, ProcessedBlock, ProcessedBlockTransactions,
    ProcessedTransaction, PROCESSED_BLOCK_SCHEMA_VERSION,
};
use alloy_primitives::{Address, Bytes, B256, U256};
use eyre::{bail, Result};
use reth_chain_query::provider::{BlockHeader, TransactionData, TransactionReceipt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::reader::TokenProcessedBlockCacheReader;
use super::writer::TokenProcessedBlockCacheWriter;

const TOKEN_BLOCK_CACHE_SCHEMA_VERSION: u32 = 4;
const TRACE_ENGINE_ID: &str = "fresh_inspector";

#[derive(Debug, Clone)]
pub struct TokenProcessedBlockCacheStore {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenProcessedBlockCacheKey {
    pub chain_id: u64,
    pub block_number: u64,
    pub block_hash: B256,
    pub tx_processor_schema_version: u32,
    pub token_cache_schema_version: u32,
    pub trace_engine: String,
    pub trace_config_hash: B256,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenProcessedBlockCacheCoverage {
    pub root: String,
    pub chain_count: usize,
    pub block_dir_count: usize,
    pub file_count: usize,
    pub total_bytes: u64,
    pub trace_engine: String,
    pub trace_config_hash: String,
    pub chains: Vec<TokenProcessedBlockCacheChainCoverage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenProcessedBlockCacheChainCoverage {
    pub chain_id: u64,
    pub path: String,
    pub block_dir_count: usize,
    pub file_count: usize,
    pub total_bytes: u64,
    pub min_block: Option<u64>,
    pub max_block: Option<u64>,
    pub ranges: Vec<TokenProcessedBlockCacheBlockRange>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenProcessedBlockCacheBlockRange {
    pub start_block: u64,
    pub end_block: u64,
    pub block_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenProcessedBlockCacheEntry {
    key: TokenProcessedBlockCacheKey,
    header: BlockHeader,
    transactions: Vec<TokenCachedTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenCachedTransaction {
    processed: SparseProcessedTransaction,
    processing_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SparseProcessedTransaction {
    hash: B256,
    block_number: u64,
    block_timestamp: u64,
    tx_index: u64,
    from_address: Address,
    to_address: Option<Address>,
    contract_address: Option<Address>,
    value: U256,
    status: bool,
    nonce: u64,
    raw_tx_type: u8,
    fees: TransactionFees,
    input: Option<Vec<u8>>,
    access_list: Option<Vec<ProcessedAccessListItem>>,
    blob_versioned_hashes: Option<Vec<B256>>,
    bribe_amount: Option<U256>,
    unique_addresses: Option<Vec<Address>>,
    erc20_contracts: Option<Vec<Address>>,
    internal_transactions: Option<Vec<InternalTransaction>>,
    erc20_transfers: Option<Vec<ERC20TransferEvent>>,
    erc20_approval_events: Option<Vec<ERC20ApprovalEvent>>,
    uniswap_v2_syncs: Option<Vec<UniswapV2SyncEvent>>,
    uniswap_v2_swaps: Option<Vec<UniswapV2SwapEvent>>,
    uniswap_v2_mints: Option<Vec<UniswapV2MintEvent>>,
    uniswap_v2_burns: Option<Vec<UniswapV2BurnEvent>>,
    uniswap_v2_pair_created_events: Option<Vec<UniswapV2PairCreatedEvent>>,
    ownership_transferred_events: Option<Vec<OwnershipTransferredEvent>>,
    ownership_transfer_started_events: Option<Vec<OwnershipTransferStartedEvent>>,
    access_control_role_granted_events: Option<Vec<AccessControlRoleGrantedEvent>>,
    access_control_role_revoked_events: Option<Vec<AccessControlRoleRevokedEvent>>,
    proxy_admin_changed_events: Option<Vec<ProxyAdminChangedEvent>>,
    contract_creation_events: Option<Vec<ContractCreationEvent>>,
    trading_enabled_events: Option<Vec<TradingEnabledEvent>>,
    trading_disabled_events: Option<Vec<TradingDisabledEvent>>,
}

impl TokenProcessedBlockCacheKey {
    pub fn new(chain_id: u64, header: &BlockHeader) -> Self {
        Self {
            chain_id,
            block_number: header.number,
            block_hash: header.hash,
            tx_processor_schema_version: PROCESSED_BLOCK_SCHEMA_VERSION,
            token_cache_schema_version: TOKEN_BLOCK_CACHE_SCHEMA_VERSION,
            trace_engine: TRACE_ENGINE_ID.to_string(),
            trace_config_hash: processed_block_trace_config_hash(true),
        }
    }
}

impl TokenProcessedBlockCacheStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn reader(&self) -> TokenProcessedBlockCacheReader {
        TokenProcessedBlockCacheReader::new(self.clone())
    }

    pub fn writer(&self, chain_id: u64) -> TokenProcessedBlockCacheWriter {
        TokenProcessedBlockCacheWriter::new(self.clone(), chain_id)
    }

    pub fn key_for_block(
        &self,
        chain_id: u64,
        block: &ProcessedBlock,
    ) -> TokenProcessedBlockCacheKey {
        TokenProcessedBlockCacheKey::new(chain_id, &block.header)
    }

    pub fn contains(&self, key: &TokenProcessedBlockCacheKey) -> bool {
        self.path_for_key(key).exists()
    }

    pub fn cached_key_for_block_number(
        &self,
        chain_id: u64,
        block_number: u64,
    ) -> Result<Option<TokenProcessedBlockCacheKey>> {
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
            let entry: TokenProcessedBlockCacheEntry = bincode::deserialize(&decoded)?;
            if entry.key.chain_id != chain_id
                || entry.key.block_number != block_number
                || entry.key.tx_processor_schema_version != PROCESSED_BLOCK_SCHEMA_VERSION
                || entry.key.token_cache_schema_version != TOKEN_BLOCK_CACHE_SCHEMA_VERSION
                || entry.key.trace_engine != TRACE_ENGINE_ID
                || entry.key.trace_config_hash != processed_block_trace_config_hash(true)
            {
                eyre::bail!(
                    "token processed block cache key mismatch for {}: found {:?}",
                    path.display(),
                    entry.key
                );
            }
            return Ok(Some(entry.key));
        }

        Ok(None)
    }

    pub fn get(&self, key: &TokenProcessedBlockCacheKey) -> Result<Option<ProcessedBlock>> {
        let path = self.path_for_key(key);
        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&path)?;
        let decoded = zstd::stream::decode_all(bytes.as_slice())?;
        let entry: TokenProcessedBlockCacheEntry = bincode::deserialize(&decoded)?;
        if entry.key != *key {
            bail!(
                "token processed block cache key mismatch for {}: expected {:?}, found {:?}",
                path.display(),
                key,
                entry.key
            );
        }
        Ok(Some(entry.into_processed_block()))
    }

    pub fn put(&self, key: &TokenProcessedBlockCacheKey, block: &ProcessedBlock) -> Result<()> {
        let path = self.path_for_key(key);
        let parent = path
            .parent()
            .ok_or_else(|| eyre::eyre!("cache path has no parent: {}", path.display()))?;
        fs::create_dir_all(parent)?;

        let bytes = bincode::serialize(&TokenProcessedBlockCacheEntry::from_block(
            key.clone(),
            block,
        ))?;
        let bytes = zstd::stream::encode_all(bytes.as_slice(), 1)?;
        let temp_path = parent.join(format!(
            ".{}.tmp-{}-{}",
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("token-processed-block"),
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

    pub fn coverage(&self) -> Result<TokenProcessedBlockCacheCoverage> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(TokenProcessedBlockCacheCoverage {
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

        Ok(TokenProcessedBlockCacheCoverage {
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
    ) -> Result<TokenProcessedBlockCacheChainCoverage> {
        let entries = match fs::read_dir(chain_path) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(TokenProcessedBlockCacheChainCoverage {
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

        Ok(TokenProcessedBlockCacheChainCoverage {
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

    fn path_for_key(&self, key: &TokenProcessedBlockCacheKey) -> PathBuf {
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

fn block_ranges(block_numbers: &[u64]) -> Vec<TokenProcessedBlockCacheBlockRange> {
    let mut ranges = Vec::new();
    let Some(&first) = block_numbers.first() else {
        return ranges;
    };

    let mut start = first;
    let mut previous = first;
    for &block_number in block_numbers.iter().skip(1) {
        if block_number != previous + 1 {
            ranges.push(TokenProcessedBlockCacheBlockRange {
                start_block: start,
                end_block: previous,
                block_count: previous - start + 1,
            });
            start = block_number;
        }
        previous = block_number;
    }

    ranges.push(TokenProcessedBlockCacheBlockRange {
        start_block: start,
        end_block: previous,
        block_count: previous - start + 1,
    });
    ranges
}

impl TokenProcessedBlockCacheEntry {
    fn from_block(key: TokenProcessedBlockCacheKey, block: &ProcessedBlock) -> Self {
        Self {
            key,
            header: block.header.clone(),
            transactions: block
                .transactions
                .iter()
                .map(|tx| TokenCachedTransaction {
                    processed: SparseProcessedTransaction::from_processed(&tx.processed),
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
                .map(TokenCachedTransaction::into_block_transaction)
                .collect(),
        }
    }
}

impl TokenCachedTransaction {
    fn into_block_transaction(self) -> ProcessedBlockTransactions {
        let processed = self.processed.into_processed();
        let metadata = metadata_from_processed_transaction(&processed);
        let receipt = receipt_from_processed_transaction(&processed);
        ProcessedBlockTransactions {
            metadata,
            receipt,
            processed,
            trace: None,
            processing_error: self.processing_error,
        }
    }
}

impl SparseProcessedTransaction {
    fn from_processed(tx: &ProcessedTransaction) -> Self {
        Self {
            hash: tx.hash,
            block_number: tx.block_number,
            block_timestamp: tx.block_timestamp,
            tx_index: tx.tx_index,
            from_address: tx.from_address,
            to_address: tx.to_address,
            contract_address: tx.contract_address,
            value: tx.value,
            status: tx.status,
            nonce: tx.nonce,
            raw_tx_type: tx.raw_tx_type,
            fees: tx.fees.clone(),
            input: option_vec(&tx.input),
            access_list: option_vec(&tx.access_list),
            blob_versioned_hashes: option_vec(&tx.blob_versioned_hashes),
            bribe_amount: option_nonzero_u256(tx.bribe_amount),
            unique_addresses: option_address_set(&tx.unique_addresses),
            erc20_contracts: option_address_set(&tx.erc20_contracts),
            internal_transactions: option_vec(&tx.internal_transactions),
            erc20_transfers: option_vec(&tx.erc20_transfers),
            erc20_approval_events: option_vec(&tx.erc20_approval_events),
            uniswap_v2_syncs: option_vec(&tx.uniswap_v2_syncs),
            uniswap_v2_swaps: option_vec(&tx.uniswap_v2_swaps),
            uniswap_v2_mints: option_vec(&tx.uniswap_v2_mints),
            uniswap_v2_burns: option_vec(&tx.uniswap_v2_burns),
            uniswap_v2_pair_created_events: option_vec(&tx.uniswap_v2_pair_created_events),
            ownership_transferred_events: option_vec(&tx.ownership_transferred_events),
            ownership_transfer_started_events: option_vec(&tx.ownership_transfer_started_events),
            access_control_role_granted_events: option_vec(&tx.access_control_role_granted_events),
            access_control_role_revoked_events: option_vec(&tx.access_control_role_revoked_events),
            proxy_admin_changed_events: option_vec(&tx.proxy_admin_changed_events),
            contract_creation_events: option_vec(&tx.contract_creation_events),
            trading_enabled_events: option_vec(&tx.trading_enabled_events),
            trading_disabled_events: option_vec(&tx.trading_disabled_events),
        }
    }

    fn into_processed(self) -> ProcessedTransaction {
        let mut tx = ProcessedTransaction::new(
            self.hash,
            self.block_number,
            self.block_timestamp,
            self.tx_index,
            self.from_address,
            self.to_address,
            self.value,
            self.status,
            self.nonce,
            self.raw_tx_type,
            Vec::new(),
        );

        tx.contract_address = self.contract_address;
        tx.fees = self.fees;
        tx.input = self.input.unwrap_or_default();
        tx.access_list = self.access_list.unwrap_or_default();
        tx.blob_versioned_hashes = self.blob_versioned_hashes.unwrap_or_default();
        tx.bribe_amount = self.bribe_amount.unwrap_or_default();
        tx.unique_addresses = option_address_vec_into_set(self.unique_addresses);
        tx.erc20_contracts = option_address_vec_into_set(self.erc20_contracts);
        tx.internal_transactions = self.internal_transactions.unwrap_or_default();
        tx.erc20_transfers = self.erc20_transfers.unwrap_or_default();
        tx.erc20_approval_events = self.erc20_approval_events.unwrap_or_default();
        tx.uniswap_v2_syncs = self.uniswap_v2_syncs.unwrap_or_default();
        tx.uniswap_v2_swaps = self.uniswap_v2_swaps.unwrap_or_default();
        tx.uniswap_v2_mints = self.uniswap_v2_mints.unwrap_or_default();
        tx.uniswap_v2_burns = self.uniswap_v2_burns.unwrap_or_default();
        tx.uniswap_v2_pair_created_events = self.uniswap_v2_pair_created_events.unwrap_or_default();
        tx.ownership_transferred_events = self.ownership_transferred_events.unwrap_or_default();
        tx.ownership_transfer_started_events =
            self.ownership_transfer_started_events.unwrap_or_default();
        tx.access_control_role_granted_events =
            self.access_control_role_granted_events.unwrap_or_default();
        tx.access_control_role_revoked_events =
            self.access_control_role_revoked_events.unwrap_or_default();
        tx.proxy_admin_changed_events = self.proxy_admin_changed_events.unwrap_or_default();
        tx.contract_creation_events = self.contract_creation_events.unwrap_or_default();
        tx.trading_enabled_events = self.trading_enabled_events.unwrap_or_default();
        tx.trading_disabled_events = self.trading_disabled_events.unwrap_or_default();

        tx
    }
}

fn metadata_from_processed_transaction(tx: &ProcessedTransaction) -> TransactionData {
    TransactionData {
        hash: tx.hash,
        block_number: tx.block_number,
        block_timestamp: tx.block_timestamp,
        tx_index: tx.tx_index,
        tx_number: 0,
        from: tx.from_address,
        to: tx.to_address,
        value: tx.value,
        input: Bytes::from(tx.input.clone()),
        gas_price: U256::ZERO,
        gas_limit: tx.fees.gas_limit,
        nonce: tx.nonce,
        transaction_type: tx.raw_tx_type,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        access_list: Vec::new(),
        blob_versioned_hashes: Vec::new(),
        max_fee_per_blob_gas: None,
        signed_authorizations: Vec::new(),
    }
}

fn receipt_from_processed_transaction(tx: &ProcessedTransaction) -> TransactionReceipt {
    TransactionReceipt {
        tx_hash: tx.hash,
        status: tx.status,
        gas_used: 0,
        logs: Vec::new(),
        cumulative_gas_used: 0,
        effective_gas_price: U256::ZERO,
        contract_address: tx.contract_address,
        blob_gas_used: None,
    }
}

fn option_vec<T: Clone>(items: &[T]) -> Option<Vec<T>> {
    if items.is_empty() {
        None
    } else {
        Some(items.to_vec())
    }
}

fn option_address_set(items: &HashSet<Address>) -> Option<Vec<Address>> {
    if items.is_empty() {
        return None;
    }
    let mut items: Vec<_> = items.iter().copied().collect();
    items.sort_unstable();
    Some(items)
}

fn option_address_vec_into_set(items: Option<Vec<Address>>) -> HashSet<Address> {
    items.unwrap_or_default().into_iter().collect()
}

fn option_nonzero_u256(value: U256) -> Option<U256> {
    if value.is_zero() {
        None
    } else {
        Some(value)
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
    use crate::tx_processor::data_models::ProcessedAccessListItem;

    #[test]
    fn round_trips_replay_fields() {
        let root = std::env::temp_dir().join(format!(
            "token-processed-block-cache-test-{}-{}",
            std::process::id(),
            monotonic_nanos()
        ));
        let store = TokenProcessedBlockCacheStore::open(&root).expect("open cache");

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
            transactions: vec![ProcessedBlockTransactions {
                metadata: metadata_from_processed_transaction(&tx),
                receipt: receipt_from_processed_transaction(&tx),
                processed: tx,
                trace: None,
                processing_error: None,
            }],
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
