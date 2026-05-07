use alloy_primitives::{Address, B256};
use async_trait::async_trait;

use crate::{BlockNumber, EncodedChainStateSnapshot, Result, TokenSnapshot};

#[async_trait]
pub trait LiveStateReader: Send + Sync {
    async fn latest_block_number(&self) -> Result<Option<BlockNumber>>;

    async fn latest_block_hash(&self) -> Result<Option<B256>>;

    async fn latest_chain_state_block_number(&self) -> Result<Option<BlockNumber>>;

    async fn read_chain_state_snapshot(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<EncodedChainStateSnapshot>>;

    async fn read_token(&self, token_address: Address) -> Result<Option<TokenSnapshot>>;

    async fn list_token_addresses(&self) -> Result<Vec<Address>>;
}
