use serde::{Deserialize, Serialize};

/// Serialized representation of a synthetic state overlay for a block.
///
/// This struct intentionally keeps the payload opaque so higher layers can
/// decide how to encode forked state deltas. For now we only record the block
/// number, the parent block, and an arbitrary byte payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateOverlaySnapshot {
    pub block_number: u64,
    pub parent_block_number: u64,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

impl StateOverlaySnapshot {
    pub fn new(block_number: u64, parent_block_number: u64, payload: Vec<u8>) -> Self {
        Self {
            block_number,
            parent_block_number,
            payload,
        }
    }
}
