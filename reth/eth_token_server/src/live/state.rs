use std::collections::BTreeSet;

use eth_token::live::LiveBlockTokenProcessor;
use eth_token::manager::LiveTokenRetentionReport;
use serde::{Deserialize, Serialize};

use crate::runs::progress::now_unix_secs;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveTrackerStatus {
    Idle,
    Warming,
    Ready,
    Stopping,
    Stopped,
    Failed,
}

impl LiveTrackerStatus {
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Warming | Self::Stopping)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiveTrackerError {
    pub block_number: Option<u64>,
    pub tx_index: Option<u64>,
    pub tx_hash: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveTrackerProgress {
    pub id: Option<String>,
    pub status: LiveTrackerStatus,
    pub history_limit: usize,
    pub warmup_start_block: Option<u64>,
    pub warmup_end_block: Option<u64>,
    pub warmup_total_blocks: u64,
    pub current_block: Option<u64>,
    pub blocks_processed: u64,
    pub txs_scanned: usize,
    pub txs_processed: usize,
    pub tx_failures: usize,
    pub token_update_reports: usize,
    pub created_tokens_unique: usize,
    pub updated_tokens_unique: usize,
    pub discovered_v2_pools_unique: usize,
    pub updated_v2_pools_unique: usize,
    pub tracked_tokens: usize,
    pub indexed_tokens: usize,
    pub indexed_v2_pools: usize,
    pub tracked_v2_pools: usize,
    pub retention_evaluated_tokens: usize,
    pub retention_dropped_tokens: usize,
    pub retention_dropped_v2_pools: usize,
    pub processed_block_cache_hits: u64,
    pub processed_block_cache_misses: u64,
    pub last_block_source: Option<String>,
    pub last_block_upstream_ms: Option<u128>,
    pub last_block_token_apply_ms: Option<u128>,
    pub last_block_cache_read_ms: Option<u128>,
    pub last_block_cache_write_ms: Option<u128>,
    pub started_at_unix_secs: Option<u64>,
    pub updated_at_unix_secs: u64,
    pub completed_at_unix_secs: Option<u64>,
    pub last_error: Option<String>,
}

impl LiveTrackerProgress {
    pub fn idle(history_limit: usize) -> Self {
        let now = now_unix_secs();
        Self {
            id: None,
            status: LiveTrackerStatus::Idle,
            history_limit,
            warmup_start_block: None,
            warmup_end_block: None,
            warmup_total_blocks: 0,
            current_block: None,
            blocks_processed: 0,
            txs_scanned: 0,
            txs_processed: 0,
            tx_failures: 0,
            token_update_reports: 0,
            created_tokens_unique: 0,
            updated_tokens_unique: 0,
            discovered_v2_pools_unique: 0,
            updated_v2_pools_unique: 0,
            tracked_tokens: 0,
            indexed_tokens: 0,
            indexed_v2_pools: 0,
            tracked_v2_pools: 0,
            retention_evaluated_tokens: 0,
            retention_dropped_tokens: 0,
            retention_dropped_v2_pools: 0,
            processed_block_cache_hits: 0,
            processed_block_cache_misses: 0,
            last_block_source: None,
            last_block_upstream_ms: None,
            last_block_token_apply_ms: None,
            last_block_cache_read_ms: None,
            last_block_cache_write_ms: None,
            started_at_unix_secs: None,
            updated_at_unix_secs: now,
            completed_at_unix_secs: None,
            last_error: None,
        }
    }

    pub fn warming(
        id: impl Into<String>,
        history_limit: usize,
        start_block: u64,
        end_block: u64,
        now: u64,
    ) -> Self {
        Self {
            id: Some(id.into()),
            status: LiveTrackerStatus::Warming,
            history_limit,
            warmup_start_block: Some(start_block),
            warmup_end_block: Some(end_block),
            warmup_total_blocks: end_block - start_block + 1,
            current_block: None,
            blocks_processed: 0,
            txs_scanned: 0,
            txs_processed: 0,
            tx_failures: 0,
            token_update_reports: 0,
            created_tokens_unique: 0,
            updated_tokens_unique: 0,
            discovered_v2_pools_unique: 0,
            updated_v2_pools_unique: 0,
            tracked_tokens: 0,
            indexed_tokens: 0,
            indexed_v2_pools: 0,
            tracked_v2_pools: 0,
            retention_evaluated_tokens: 0,
            retention_dropped_tokens: 0,
            retention_dropped_v2_pools: 0,
            processed_block_cache_hits: 0,
            processed_block_cache_misses: 0,
            last_block_source: None,
            last_block_upstream_ms: None,
            last_block_token_apply_ms: None,
            last_block_cache_read_ms: None,
            last_block_cache_write_ms: None,
            started_at_unix_secs: Some(now),
            updated_at_unix_secs: now,
            completed_at_unix_secs: None,
            last_error: None,
        }
    }
}

#[derive(Debug)]
pub struct LiveTrackerState {
    pub processor: LiveBlockTokenProcessor,
    pub progress: LiveTrackerProgress,
    pub errors: Vec<LiveTrackerError>,
    pub created_tokens: BTreeSet<String>,
    pub updated_tokens: BTreeSet<String>,
    pub discovered_v2_pools: BTreeSet<String>,
    pub updated_v2_pools: BTreeSet<String>,
    pub last_retention_report: Option<LiveTokenRetentionReport>,
}

impl LiveTrackerState {
    pub fn idle(history_limit: usize) -> Self {
        Self {
            processor: LiveBlockTokenProcessor::new(history_limit),
            progress: LiveTrackerProgress::idle(history_limit),
            errors: Vec::new(),
            created_tokens: BTreeSet::new(),
            updated_tokens: BTreeSet::new(),
            discovered_v2_pools: BTreeSet::new(),
            updated_v2_pools: BTreeSet::new(),
            last_retention_report: None,
        }
    }

    pub fn warming(id: String, history_limit: usize, start_block: u64, end_block: u64) -> Self {
        let now = now_unix_secs();
        Self {
            processor: LiveBlockTokenProcessor::new(history_limit),
            progress: LiveTrackerProgress::warming(id, history_limit, start_block, end_block, now),
            errors: Vec::new(),
            created_tokens: BTreeSet::new(),
            updated_tokens: BTreeSet::new(),
            discovered_v2_pools: BTreeSet::new(),
            updated_v2_pools: BTreeSet::new(),
            last_retention_report: None,
        }
    }
}
