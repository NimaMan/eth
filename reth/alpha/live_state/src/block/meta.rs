use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

use crate::{BlockNumber, ChainId, TimestampUnixSecs};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockMeta {
    pub chain_id: ChainId,
    pub block_number: BlockNumber,
    pub block_hash: B256,
    pub parent_hash: B256,
    pub timestamp_unix_secs: TimestampUnixSecs,
}

impl BlockMeta {
    pub const fn new(
        chain_id: ChainId,
        block_number: BlockNumber,
        block_hash: B256,
        parent_hash: B256,
        timestamp_unix_secs: TimestampUnixSecs,
    ) -> Self {
        Self {
            chain_id,
            block_number,
            block_hash,
            parent_hash,
            timestamp_unix_secs,
        }
    }
}
