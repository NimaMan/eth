use alloy_primitives::{Address, B256};
use eth_live_state::BlockNumber;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LiveFeedEvent {
    BlockProcessed(BlockProcessedEvent),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockProcessedEvent {
    pub block_number: BlockNumber,
    pub block_hash: B256,
    pub parent_hash: B256,
    pub processed_transaction_count: usize,
    pub updated_tokens: Vec<Address>,
    pub removed_tokens: Vec<Address>,
    pub chain_state_available: bool,
}

impl BlockProcessedEvent {
    pub fn updated_token_count(&self) -> usize {
        self.updated_tokens.len()
    }
}
