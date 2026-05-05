use alloy_primitives::Address;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    BlockReadyNotification, EncodedChainStateSnapshot, ProcessedBlockSnapshot, Result,
    TokenSnapshot,
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SnapshotWriteOptions {
    pub ttl_secs: Option<u64>,
}

#[async_trait]
pub trait LiveStateWriter: Send + Sync {
    async fn write_block(&self, block: ProcessedBlockSnapshot) -> Result<()>;

    async fn mark_block_ready(&self, notification: BlockReadyNotification) -> Result<()>;

    async fn write_chain_state_snapshot(&self, snapshot: EncodedChainStateSnapshot) -> Result<()>;

    async fn write_token(
        &self,
        snapshot: TokenSnapshot,
        options: SnapshotWriteOptions,
    ) -> Result<()>;

    async fn delete_token(&self, token_address: Address) -> Result<()>;
}
