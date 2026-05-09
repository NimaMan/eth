use alloy_primitives::B256;
use reth_revm::db::Cache;
use serde::{Deserialize, Serialize};

/// Serialized representation of a synthetic chain-state snapshot for a block.
///
/// The snapshot stores the cumulative REVM cache overlay from
/// `base_block_number` through `block_number`. It does not duplicate the full
/// MDBX state; restore creates a read-only historical DB at the base block and
/// applies this cache on top.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStateSnapshot {
    pub schema_version: u16,
    pub base_block_number: u64,
    pub block_number: u64,
    pub block_hash: B256,
    pub parent_hash: B256,
    pub cache: Cache,
}

impl ChainStateSnapshot {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn new(
        base_block_number: u64,
        block_number: u64,
        block_hash: B256,
        parent_hash: B256,
        cache: Cache,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            base_block_number,
            block_number,
            block_hash,
            parent_hash,
            cache,
        }
    }

    pub fn account_count(&self) -> usize {
        self.cache.accounts.len()
    }

    pub fn contract_count(&self) -> usize {
        self.cache.contracts.len()
    }

    pub fn log_count(&self) -> usize {
        self.cache.logs.len()
    }
}
