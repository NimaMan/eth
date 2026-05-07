use async_trait::async_trait;

use crate::{LiveFeedBlockInput, Result};

#[async_trait]
pub trait ProcessedBlockSource: Send + Sync {
    async fn next_block(&self) -> Result<Option<LiveFeedBlockInput>>;
}
