use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use eth_block_tx_rank::MinedBlockFeeSample;
use serde::Serialize;
use tx_processor::{LiveBlockStateFrame, ProcessedBlock};

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

#[derive(Clone, Debug, Serialize)]
pub struct RecentLiveStateFrame {
    pub frame: LiveBlockStateFrame,
    pub applied_at_unix_ms: u64,
}

#[derive(Clone, Debug)]
pub struct RecentLiveStateFrames {
    inner: Arc<RwLock<VecDeque<RecentLiveStateFrame>>>,
    capacity: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecentLiveFeeSample {
    pub sample: MinedBlockFeeSample,
    pub applied_at_unix_ms: u64,
}

#[derive(Clone, Debug)]
pub struct RecentLiveFeeSamples {
    inner: Arc<RwLock<VecDeque<RecentLiveFeeSample>>>,
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

impl RecentLiveFeeSamples {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(VecDeque::with_capacity(capacity.max(1)))),
            capacity: capacity.max(1),
        }
    }

    pub fn record(&self, sample: MinedBlockFeeSample) {
        let block_number = sample.block_number;
        let mut samples = self
            .inner
            .write()
            .expect("recent live fee samples lock poisoned");
        if let Some(existing) = samples
            .iter_mut()
            .find(|entry| entry.sample.block_number == block_number)
        {
            existing.sample = sample;
            existing.applied_at_unix_ms = now_unix_ms();
            return;
        }

        samples.push_front(RecentLiveFeeSample {
            sample,
            applied_at_unix_ms: now_unix_ms(),
        });
        while samples.len() > self.capacity {
            samples.pop_back();
        }
    }

    pub fn latest(&self, limit: usize) -> Vec<RecentLiveFeeSample> {
        let samples = self
            .inner
            .read()
            .expect("recent live fee samples lock poisoned");
        samples.iter().take(limit).cloned().collect()
    }
}

pub fn mined_fee_sample_from_processed_block(block: &ProcessedBlock) -> MinedBlockFeeSample {
    let rows = block
        .transactions
        .iter()
        .map(|tx| (tx.metadata.clone(), tx.receipt.clone()))
        .collect::<Vec<_>>();
    MinedBlockFeeSample::from_provider_rows(block.header.clone(), rows)
}

impl RecentLiveStateFrames {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(VecDeque::with_capacity(capacity.max(1)))),
            capacity: capacity.max(1),
        }
    }

    pub fn record(&self, frame: LiveBlockStateFrame) {
        let block_number = frame.header.number;
        let mut frames = self
            .inner
            .write()
            .expect("recent live state frames lock poisoned");
        if let Some(existing) = frames
            .iter_mut()
            .find(|entry| entry.frame.header.number == block_number)
        {
            existing.frame = frame;
            existing.applied_at_unix_ms = now_unix_ms();
            return;
        }

        frames.push_front(RecentLiveStateFrame {
            frame,
            applied_at_unix_ms: now_unix_ms(),
        });
        while frames.len() > self.capacity {
            frames.pop_back();
        }
    }

    pub fn latest(&self) -> Option<RecentLiveStateFrame> {
        self.inner
            .read()
            .expect("recent live state frames lock poisoned")
            .front()
            .cloned()
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
