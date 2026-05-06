use std::time::Instant;

use crate::ProcessedBlock;
use eyre::Result;

use super::store::{TokenProcessedBlockCacheKey, TokenProcessedBlockCacheStore};

#[derive(Debug, Clone)]
pub struct TokenProcessedBlockCacheWriter {
    store: TokenProcessedBlockCacheStore,
    chain_id: u64,
}

#[derive(Debug, Clone)]
pub struct TokenProcessedBlockCacheWrite {
    pub key: TokenProcessedBlockCacheKey,
    pub write_ms: u128,
}

impl TokenProcessedBlockCacheWriter {
    pub fn new(store: TokenProcessedBlockCacheStore, chain_id: u64) -> Self {
        Self { store, chain_id }
    }

    pub fn write_processed_block(
        &self,
        block: &ProcessedBlock,
    ) -> Result<TokenProcessedBlockCacheWrite> {
        let key = self.store.key_for_block(self.chain_id, block);
        let write_started = Instant::now();
        self.store.put(&key, block)?;
        Ok(TokenProcessedBlockCacheWrite {
            key,
            write_ms: write_started.elapsed().as_millis(),
        })
    }
}
