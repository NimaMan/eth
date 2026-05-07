use std::time::Instant;

use crate::ProcessedBlock;
use eyre::Result;

use super::store::{ProcessedBlockDiskCacheKey, ProcessedBlockDiskCacheStore};

#[derive(Debug, Clone)]
pub struct ProcessedBlockDiskCacheWriter {
    store: ProcessedBlockDiskCacheStore,
    chain_id: u64,
}

#[derive(Debug, Clone)]
pub struct ProcessedBlockDiskCacheWrite {
    pub key: ProcessedBlockDiskCacheKey,
    pub write_ms: u128,
}

impl ProcessedBlockDiskCacheWriter {
    pub fn new(store: ProcessedBlockDiskCacheStore, chain_id: u64) -> Self {
        Self { store, chain_id }
    }

    pub fn write_processed_block(
        &self,
        block: &ProcessedBlock,
    ) -> Result<ProcessedBlockDiskCacheWrite> {
        let key = self.store.key_for_block(self.chain_id, block);
        let write_started = Instant::now();
        self.store.put(&key, block)?;
        Ok(ProcessedBlockDiskCacheWrite {
            key,
            write_ms: write_started.elapsed().as_millis(),
        })
    }
}
