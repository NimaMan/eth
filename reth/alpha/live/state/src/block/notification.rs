use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

use crate::BlockNumber;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockReadyNotification {
    pub block_number: BlockNumber,
    pub block_hash: B256,
    pub processed_transaction_count: usize,
    pub updated_token_count: usize,
}

impl BlockReadyNotification {
    pub const fn new(
        block_number: BlockNumber,
        block_hash: B256,
        processed_transaction_count: usize,
        updated_token_count: usize,
    ) -> Self {
        Self {
            block_number,
            block_hash,
            processed_transaction_count,
            updated_token_count,
        }
    }
}
