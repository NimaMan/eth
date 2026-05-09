use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use alloy_primitives::{keccak256, B256};
use eyre::{bail, Result};
use serde::{Deserialize, Serialize};
use tx_simulator::block_simulation::BlockTraceEngine;

use crate::block_processor::{ProcessedBlock, PROCESSED_BLOCK_SCHEMA_VERSION};
use crate::processed_block_provider::COMPACT_PROCESSED_TRANSACTION_SCHEMA_VERSION;
use crate::tx_processor::data_models::ProcessedTransaction;

/// Default number of blocks to retain in the cache (~2 days on Ethereum mainnet).
const BLOCK_RETENTION: u64 = 10_000;

/// In-memory cache of processed blocks keyed by block number.
#[derive(Default)]
pub struct ProcessedBlockCache {
    blocks: BTreeMap<u64, Arc<ProcessedBlock>>,
    transactions: HashMap<B256, ProcessedTransaction>,
}

impl ProcessedBlockCache {
    pub fn new() -> Self {
        Self {
            blocks: BTreeMap::new(),
            transactions: HashMap::new(),
        }
    }

    /// Insert a batch of processed blocks into the cache.
    pub fn insert_many(&mut self, blocks: Vec<ProcessedBlock>) {
        for block in blocks {
            self.insert(block);
        }
    }

    /// Insert a processed block into the cache.
    pub fn insert(&mut self, block: ProcessedBlock) {
        let number = block.header.number;
        let block_arc = Arc::new(block);

        if let Some(existing) = self.blocks.insert(number, block_arc.clone()) {
            // If we replaced an existing block, remove its transactions from the index.
            for tx in &existing.transactions {
                self.transactions.remove(&tx.processed.hash);
            }
        }

        for tx in &block_arc.transactions {
            self.transactions
                .insert(tx.processed.hash, tx.processed.clone());
        }

        self.prune_before(number.saturating_sub(BLOCK_RETENTION));
    }

    /// Returns the list of blocks not yet cached.
    pub fn missing_blocks(&self, block_numbers: &[u64]) -> Vec<u64> {
        block_numbers
            .iter()
            .copied()
            .filter(|number| !self.blocks.contains_key(number))
            .collect()
    }

    /// Retrieve cached block if present.
    pub fn get(&self, block_number: u64) -> Option<Arc<ProcessedBlock>> {
        self.blocks.get(&block_number).cloned()
    }

    /// Iterate over all cached blocks in numeric order.
    pub fn iter(&self) -> impl Iterator<Item = (&u64, &Arc<ProcessedBlock>)> {
        self.blocks.iter()
    }

    /// Retrieve processed transaction by hash if present in cache.
    pub fn get_transaction(&self, tx_hash: &B256) -> Option<ProcessedTransaction> {
        self.transactions.get(tx_hash).cloned()
    }

    fn prune_before(&mut self, min_block: u64) {
        if self.blocks.is_empty() {
            return;
        }

        let to_remove: Vec<u64> = self
            .blocks
            .range(..min_block)
            .map(|(block_number, _)| *block_number)
            .collect();

        for block_number in to_remove {
            if let Some(block) = self.blocks.remove(&block_number) {
                for tx in &block.transactions {
                    self.transactions.remove(&tx.processed.hash);
                }
            }
        }
    }
}

/// Persistent cache key for a complete processed block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessedBlockCacheKey {
    pub chain_id: u64,
    pub block_number: u64,
    pub block_hash: B256,
    pub processor_schema_version: u32,
    pub trace_engine: String,
    pub trace_config_hash: B256,
}

impl ProcessedBlockCacheKey {
    pub fn new(
        chain_id: u64,
        block_number: u64,
        block_hash: B256,
        trace_engine: BlockTraceEngine,
        trace_config_hash: B256,
    ) -> Self {
        Self::with_schema_version(
            chain_id,
            block_number,
            block_hash,
            PROCESSED_BLOCK_SCHEMA_VERSION,
            trace_engine,
            trace_config_hash,
        )
    }

    pub fn with_schema_version(
        chain_id: u64,
        block_number: u64,
        block_hash: B256,
        processor_schema_version: u32,
        trace_engine: BlockTraceEngine,
        trace_config_hash: B256,
    ) -> Self {
        Self {
            chain_id,
            block_number,
            block_hash,
            processor_schema_version,
            trace_engine: trace_engine_id(trace_engine).to_string(),
            trace_config_hash,
        }
    }
}

/// Persistent on-disk processed block cache.
#[derive(Debug, Clone)]
pub struct ProcessedBlockCacheStore {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessedBlockCacheEntry {
    key: ProcessedBlockCacheKey,
    block_json: Vec<u8>,
}

impl ProcessedBlockCacheStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn get(&self, key: &ProcessedBlockCacheKey) -> Result<Option<ProcessedBlock>> {
        let path = self.path_for_key(key);
        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&path)?;
        let entry: ProcessedBlockCacheEntry = bincode::deserialize(&bytes)?;
        if entry.key != *key {
            bail!(
                "processed block cache key mismatch for {}: expected {:?}, found {:?}",
                path.display(),
                key,
                entry.key
            );
        }
        Ok(Some(serde_json::from_slice(&entry.block_json)?))
    }

    pub fn put(&self, key: &ProcessedBlockCacheKey, block: &ProcessedBlock) -> Result<()> {
        let path = self.path_for_key(key);
        let parent = path
            .parent()
            .ok_or_else(|| eyre::eyre!("cache path has no parent: {}", path.display()))?;
        fs::create_dir_all(parent)?;

        let entry = ProcessedBlockCacheEntry {
            key: key.clone(),
            block_json: serde_json::to_vec(block)?,
        };
        let bytes = bincode::serialize(&entry)?;
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

    pub fn remove_stale_number(&self, chain_id: u64, block_number: u64) -> Result<()> {
        let path = self
            .root
            .join(format!("chain-{chain_id}"))
            .join(format!("block-{block_number}"));
        match fs::remove_dir_all(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn prune_chain_to_recent_blocks(&self, chain_id: u64, retain_blocks: u64) -> Result<usize> {
        let retain_blocks = usize::try_from(retain_blocks).unwrap_or(usize::MAX);
        let chain_path = self.root.join(format!("chain-{chain_id}"));
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
            self.remove_stale_number(chain_id, *block_number)?;
            removed += 1;
        }
        Ok(removed)
    }

    fn path_for_key(&self, key: &ProcessedBlockCacheKey) -> PathBuf {
        self.root
            .join(format!("chain-{}", key.chain_id))
            .join(format!("block-{}", key.block_number))
            .join(format!("schema-{}", key.processor_schema_version))
            .join(format!("engine-{}", key.trace_engine))
            .join(format!(
                "config-{}",
                b256_path_component(key.trace_config_hash)
            ))
            .join(format!("{}.bin", b256_path_component(key.block_hash)))
    }
}

pub fn processed_block_trace_config_hash(include_traces: bool) -> B256 {
    let config = format!(
        "include_traces={include_traces};tracer=callTracer;version=2;compact_tx_schema={COMPACT_PROCESSED_TRANSACTION_SCHEMA_VERSION}"
    );
    keccak256(config.as_bytes())
}

pub fn trace_engine_id(trace_engine: BlockTraceEngine) -> &'static str {
    match trace_engine {
        BlockTraceEngine::FreshInspector => "fresh_inspector",
        BlockTraceEngine::RethFusedCallTracer => "reth_fused_call_tracer",
        BlockTraceEngine::RethDebug => "reth_debug",
    }
}

fn b256_path_component(value: B256) -> String {
    format!("{value:#x}")
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
    use alloy_primitives::{Address, Bytes, U256};
    use reth_chain_query::provider::{
        BlockHeader, CallFrame, CallType, TransactionMetadata, TransactionReceipt, TransactionTrace,
    };

    #[test]
    fn processed_block_bincode_round_trips_with_trace() {
        let block = dummy_processed_block();

        let bytes = bincode::serialize(&block).expect("serialize processed block");
        let decoded: ProcessedBlock =
            bincode::deserialize(&bytes).expect("deserialize processed block");

        assert_eq!(decoded.header.hash, block.header.hash);
        assert_eq!(decoded.transactions.len(), 1);
        let trace = decoded.transactions[0]
            .trace
            .as_ref()
            .expect("trace should round trip");
        assert_eq!(trace.call_frame.call_type, CallType::Call);
        assert_eq!(trace.call_frame.subcalls.len(), 1);
    }

    #[test]
    fn cache_key_changes_on_all_invalidation_inputs() {
        let base = ProcessedBlockCacheKey::new(
            1,
            10,
            B256::repeat_byte(1),
            BlockTraceEngine::FreshInspector,
            B256::repeat_byte(2),
        );

        assert_ne!(
            base,
            ProcessedBlockCacheKey::new(
                2,
                10,
                B256::repeat_byte(1),
                BlockTraceEngine::FreshInspector,
                B256::repeat_byte(2),
            )
        );
        assert_ne!(
            base,
            ProcessedBlockCacheKey::new(
                1,
                11,
                B256::repeat_byte(1),
                BlockTraceEngine::FreshInspector,
                B256::repeat_byte(2),
            )
        );
        assert_ne!(
            base,
            ProcessedBlockCacheKey::new(
                1,
                10,
                B256::repeat_byte(3),
                BlockTraceEngine::FreshInspector,
                B256::repeat_byte(2),
            )
        );
        assert_ne!(
            base,
            ProcessedBlockCacheKey::with_schema_version(
                1,
                10,
                B256::repeat_byte(1),
                PROCESSED_BLOCK_SCHEMA_VERSION + 1,
                BlockTraceEngine::FreshInspector,
                B256::repeat_byte(2),
            )
        );
        assert_ne!(
            base,
            ProcessedBlockCacheKey::new(
                1,
                10,
                B256::repeat_byte(1),
                BlockTraceEngine::RethFusedCallTracer,
                B256::repeat_byte(2),
            )
        );
        assert_ne!(
            base,
            ProcessedBlockCacheKey::new(
                1,
                10,
                B256::repeat_byte(1),
                BlockTraceEngine::FreshInspector,
                B256::repeat_byte(4),
            )
        );
    }

    #[test]
    fn persistent_cache_store_round_trips_processed_block() {
        let root = std::env::temp_dir().join(format!(
            "processed-block-cache-test-{}-{}",
            std::process::id(),
            monotonic_nanos()
        ));
        let store = ProcessedBlockCacheStore::open(&root).expect("open cache");
        let block = dummy_processed_block();
        let key = ProcessedBlockCacheKey::new(
            1,
            block.header.number,
            block.header.hash,
            BlockTraceEngine::FreshInspector,
            processed_block_trace_config_hash(true),
        );

        assert!(store.get(&key).expect("read empty cache").is_none());
        store.put(&key, &block).expect("write cache");
        let decoded = store
            .get(&key)
            .expect("read cache")
            .expect("cache should contain block");

        assert_eq!(decoded.header.hash, block.header.hash);
        assert_eq!(
            decoded.transactions[0].metadata.hash,
            block.transactions[0].metadata.hash
        );

        store
            .remove_stale_number(key.chain_id, key.block_number)
            .expect("remove block cache");
        assert!(store.get(&key).expect("read removed cache").is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prune_chain_to_recent_blocks_keeps_newest_block_numbers() {
        let root = std::env::temp_dir().join(format!(
            "processed-block-cache-prune-test-{}-{}",
            std::process::id(),
            monotonic_nanos()
        ));
        let store = ProcessedBlockCacheStore::open(&root).expect("open cache");
        let mut keys = Vec::new();

        for block_number in 10..14 {
            let mut block = dummy_processed_block();
            block.header.number = block_number;
            block.header.hash = B256::repeat_byte(block_number as u8);
            let key = ProcessedBlockCacheKey::new(
                1,
                block.header.number,
                block.header.hash,
                BlockTraceEngine::FreshInspector,
                processed_block_trace_config_hash(true),
            );
            store.put(&key, &block).expect("write cache");
            keys.push(key);
        }

        let removed = store
            .prune_chain_to_recent_blocks(1, 2)
            .expect("prune cache");
        assert_eq!(removed, 2);
        assert!(store.get(&keys[0]).expect("read old cache").is_none());
        assert!(store.get(&keys[1]).expect("read old cache").is_none());
        assert!(store.get(&keys[2]).expect("read kept cache").is_some());
        assert!(store.get(&keys[3]).expect("read kept cache").is_some());

        let _ = fs::remove_dir_all(root);
    }

    fn dummy_processed_block() -> ProcessedBlock {
        let header = BlockHeader {
            number: 10,
            hash: B256::repeat_byte(1),
            parent_hash: B256::repeat_byte(2),
            timestamp: 12,
            gas_limit: 30_000_000,
            gas_used: 21_000,
            base_fee_per_gas: Some(1),
            withdrawals_root: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
            requests_hash: None,
            block_access_list_hash: None,
            slot_number: None,
        };
        let metadata = TransactionMetadata {
            hash: B256::repeat_byte(3),
            block_number: header.number,
            block_timestamp: header.timestamp,
            tx_index: 0,
            tx_number: 0,
            from: Address::repeat_byte(4),
            to: Some(Address::repeat_byte(5)),
            value: U256::from(10),
            input: Bytes::from(vec![0x12, 0x34]),
            gas_price: U256::from(1),
            gas_limit: 21_000,
            nonce: 7,
            transaction_type: 2,
            max_fee_per_gas: Some(U256::from(2)),
            max_priority_fee_per_gas: Some(U256::from(1)),
            access_list: Vec::new(),
            blob_versioned_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            signed_authorizations: Vec::new(),
        };
        let receipt = TransactionReceipt {
            tx_hash: metadata.hash,
            status: true,
            gas_used: 21_000,
            logs: Vec::new(),
            cumulative_gas_used: 21_000,
            effective_gas_price: U256::from(1),
            contract_address: None,
            blob_gas_used: None,
        };
        let trace = TransactionTrace {
            call_frame: CallFrame {
                from: metadata.from,
                to: metadata.to,
                value: metadata.value,
                input: metadata.input.clone(),
                output: Bytes::from(vec![0xab]),
                gas_used: 21_000,
                gas_limit: 21_000,
                depth: 0,
                call_type: CallType::Call,
                subcalls: vec![CallFrame {
                    from: metadata.from,
                    to: metadata.to,
                    value: U256::from(1),
                    input: Bytes::new(),
                    output: Bytes::new(),
                    gas_used: 1_000,
                    gas_limit: 2_000,
                    depth: 1,
                    call_type: CallType::StaticCall,
                    subcalls: Vec::new(),
                }],
            },
            gas_used: 21_000,
            output: Bytes::from(vec![0xab]),
            error: None,
        };
        let processed = ProcessedTransaction::new(
            metadata.hash,
            metadata.block_number,
            metadata.block_timestamp,
            metadata.tx_index,
            metadata.from,
            metadata.to,
            metadata.value,
            receipt.status,
            metadata.nonce,
            metadata.transaction_type,
            metadata.input.clone().to_vec(),
        );

        ProcessedBlock {
            header,
            transactions: vec![crate::block_processor::ProcessedBlockTransactions {
                metadata,
                receipt,
                processed,
                trace: Some(trace),
                processing_error: None,
            }],
        }
    }
}
