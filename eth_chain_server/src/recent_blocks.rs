use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct RecentProcessedBlock {
    pub block_number: u64,
    pub block_hash: String,
    pub applied_at_unix_ms: u64,
}

#[derive(Clone, Debug)]
pub struct RecentLiveBlocks {
    inner: Arc<RwLock<VecDeque<RecentProcessedBlock>>>,
    capacity: usize,
}

impl RecentLiveBlocks {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(VecDeque::with_capacity(capacity.max(1)))),
            capacity: capacity.max(1),
        }
    }

    pub fn record(&self, block_number: u64, block_hash: impl Into<String>) {
        let mut blocks = self
            .inner
            .write()
            .expect("recent live blocks lock poisoned");
        if let Some(existing) = blocks
            .iter_mut()
            .find(|block| block.block_number == block_number)
        {
            existing.block_hash = block_hash.into();
            existing.applied_at_unix_ms = now_unix_ms();
            return;
        }

        blocks.push_front(RecentProcessedBlock {
            block_number,
            block_hash: block_hash.into(),
            applied_at_unix_ms: now_unix_ms(),
        });
        while blocks.len() > self.capacity {
            blocks.pop_back();
        }
    }

    pub fn latest(&self, limit: usize) -> Vec<RecentProcessedBlock> {
        let blocks = self.inner.read().expect("recent live blocks lock poisoned");
        blocks.iter().take(limit).cloned().collect()
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
