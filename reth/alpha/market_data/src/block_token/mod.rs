use alloy_primitives::Address;
use async_trait::async_trait;
use eth_live_state::{ProcessedBlockSnapshot, TokenSnapshot};

use crate::Result;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockTokenUpdate {
    pub updated_tokens: Vec<TokenSnapshot>,
    pub removed_tokens: Vec<Address>,
}

impl BlockTokenUpdate {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn updated_token_addresses(&self) -> Vec<Address> {
        self.updated_tokens
            .iter()
            .map(|token| token.contract_address)
            .collect()
    }
}

#[async_trait]
pub trait BlockTokenProcessor: Send + Sync {
    async fn process_block_tokens(
        &self,
        block: &ProcessedBlockSnapshot,
    ) -> Result<BlockTokenUpdate>;
}
