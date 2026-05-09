use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};

use crate::{BlockNumber, TimestampUnixSecs};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenSnapshotIndexEntry {
    pub address: Address,
    pub latest_block_number: Option<BlockNumber>,
    pub latest_block_hash: Option<B256>,
    pub updated_at_unix_secs: Option<TimestampUnixSecs>,
}

impl TokenSnapshotIndexEntry {
    pub const fn new(
        address: Address,
        latest_block_number: Option<BlockNumber>,
        latest_block_hash: Option<B256>,
        updated_at_unix_secs: Option<TimestampUnixSecs>,
    ) -> Self {
        Self {
            address,
            latest_block_number,
            latest_block_hash,
            updated_at_unix_secs,
        }
    }
}
