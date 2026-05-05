use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

use crate::BlockNumber;

pub const CHAIN_STATE_SNAPSHOT_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainStateCodec {
    RethRevmCacheBincodeV1,
    Json,
    Opaque(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChainStateSnapshotStats {
    pub account_count: Option<usize>,
    pub contract_count: Option<usize>,
    pub log_count: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EncodedChainStateSnapshot {
    pub schema_version: u16,
    pub codec: ChainStateCodec,
    pub base_block_number: BlockNumber,
    pub block_number: BlockNumber,
    pub block_hash: B256,
    pub parent_hash: B256,
    /// Codec-specific snapshot bytes.
    ///
    /// This is intentionally opaque to keep `eth_live_state` independent from
    /// `tx_simulator` and REVM internals. Redis backends can store these bytes
    /// directly under the canonical snapshot key when preserving the current
    /// simulator format.
    pub payload: Vec<u8>,
    pub stats: Option<ChainStateSnapshotStats>,
}

impl EncodedChainStateSnapshot {
    pub fn new_reth_revm_cache_bincode_v1(
        base_block_number: BlockNumber,
        block_number: BlockNumber,
        block_hash: B256,
        parent_hash: B256,
        payload: Vec<u8>,
        stats: Option<ChainStateSnapshotStats>,
    ) -> Self {
        Self {
            schema_version: CHAIN_STATE_SNAPSHOT_SCHEMA_VERSION,
            codec: ChainStateCodec::RethRevmCacheBincodeV1,
            base_block_number,
            block_number,
            block_hash,
            parent_hash,
            payload,
            stats,
        }
    }

    pub fn payload_len(&self) -> usize {
        self.payload.len()
    }
}
