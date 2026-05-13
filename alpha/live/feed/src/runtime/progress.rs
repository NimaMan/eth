use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::time::now_unix_secs;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveTokenStatus {
    Idle,
    Warming,
    Live,
    Stopping,
    Stopped,
    Failed,
}

impl LiveTokenStatus {
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Warming | Self::Live | Self::Stopping)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenError {
    pub block_number: Option<u64>,
    pub tx_index: Option<u64>,
    pub tx_hash: Option<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, String>,
}

impl LiveTokenError {
    pub fn new(
        block_number: Option<u64>,
        tx_index: Option<u64>,
        tx_hash: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            block_number,
            tx_index,
            tx_hash,
            message: message.into(),
            detail: None,
            context: BTreeMap::new(),
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl ToString) -> Self {
        self.context.insert(key.into(), value.to_string());
        self
    }

    pub fn with_context_map(mut self, context: BTreeMap<String, String>) -> Self {
        self.context.extend(context);
        self
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct StartLiveTokenRuntimeRequest {
    #[serde(default)]
    pub start_block: Option<u64>,
    #[serde(default)]
    pub end_block: Option<u64>,
    #[serde(default)]
    pub warmup_blocks: Option<u64>,
    #[serde(default)]
    pub history_limit: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedLiveTokenRuntimeRequest {
    pub id: String,
    pub start_block: u64,
    pub end_block: u64,
    pub warmup_blocks: u64,
    pub history_limit: usize,
}

impl ResolvedLiveTokenRuntimeRequest {
    pub fn block_count(&self) -> u64 {
        self.end_block - self.start_block + 1
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveTokenProgress {
    pub id: Option<String>,
    pub status: LiveTokenStatus,
    pub history_limit: usize,
    pub warmup_start_block: Option<u64>,
    pub warmup_end_block: Option<u64>,
    pub warmup_total_blocks: u64,
    pub current_block: Option<u64>,
    pub blocks_processed: u64,
    pub live_blocks_processed: u64,
    pub txs_scanned: usize,
    pub txs_processed: usize,
    pub tx_failures: usize,
    pub token_update_reports: usize,
    pub created_tokens_unique: usize,
    pub updated_tokens_unique: usize,
    pub discovered_v2_pools_unique: usize,
    pub updated_v2_pools_unique: usize,
    pub discovered_v3_pools_unique: usize,
    pub updated_v3_pools_unique: usize,
    pub discovered_v4_pools_unique: usize,
    pub updated_v4_pools_unique: usize,
    pub tracked_tokens: usize,
    pub indexed_tokens: usize,
    pub indexed_pools: usize,
    pub tracked_pools: usize,
    pub indexed_v2_pools: usize,
    pub tracked_v2_pools: usize,
    pub indexed_v3_pools: usize,
    pub tracked_v3_pools: usize,
    pub indexed_v4_pools: usize,
    pub tracked_v4_pools: usize,
    pub retention_evaluated_tokens: usize,
    pub retention_dropped_tokens: usize,
    pub retention_dropped_v2_pools: usize,
    pub processed_block_disk_cache_hits: u64,
    pub processed_block_disk_cache_misses: u64,
    pub live_stream_events: u64,
    pub live_gap_blocks_caught_up: u64,
    pub last_stream_id: Option<String>,
    pub last_block_source: Option<String>,
    pub last_block_upstream_ms: Option<u128>,
    pub last_block_token_apply_ms: Option<u128>,
    pub last_block_disk_cache_read_ms: Option<u128>,
    pub last_block_disk_cache_write_ms: Option<u128>,
    pub started_at_unix_secs: Option<u64>,
    pub live_at_unix_secs: Option<u64>,
    pub updated_at_unix_secs: u64,
    pub completed_at_unix_secs: Option<u64>,
    pub last_error: Option<String>,
}

impl LiveTokenProgress {
    pub fn idle(history_limit: usize) -> Self {
        let now = now_unix_secs();
        Self {
            id: None,
            status: LiveTokenStatus::Idle,
            history_limit,
            warmup_start_block: None,
            warmup_end_block: None,
            warmup_total_blocks: 0,
            current_block: None,
            blocks_processed: 0,
            live_blocks_processed: 0,
            txs_scanned: 0,
            txs_processed: 0,
            tx_failures: 0,
            token_update_reports: 0,
            created_tokens_unique: 0,
            updated_tokens_unique: 0,
            discovered_v2_pools_unique: 0,
            updated_v2_pools_unique: 0,
            discovered_v3_pools_unique: 0,
            updated_v3_pools_unique: 0,
            discovered_v4_pools_unique: 0,
            updated_v4_pools_unique: 0,
            tracked_tokens: 0,
            indexed_tokens: 0,
            indexed_pools: 0,
            tracked_pools: 0,
            indexed_v2_pools: 0,
            tracked_v2_pools: 0,
            indexed_v3_pools: 0,
            tracked_v3_pools: 0,
            indexed_v4_pools: 0,
            tracked_v4_pools: 0,
            retention_evaluated_tokens: 0,
            retention_dropped_tokens: 0,
            retention_dropped_v2_pools: 0,
            processed_block_disk_cache_hits: 0,
            processed_block_disk_cache_misses: 0,
            live_stream_events: 0,
            live_gap_blocks_caught_up: 0,
            last_stream_id: None,
            last_block_source: None,
            last_block_upstream_ms: None,
            last_block_token_apply_ms: None,
            last_block_disk_cache_read_ms: None,
            last_block_disk_cache_write_ms: None,
            started_at_unix_secs: None,
            live_at_unix_secs: None,
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
            status: LiveTokenStatus::Warming,
            history_limit,
            warmup_start_block: Some(start_block),
            warmup_end_block: Some(end_block),
            warmup_total_blocks: end_block - start_block + 1,
            current_block: None,
            blocks_processed: 0,
            live_blocks_processed: 0,
            txs_scanned: 0,
            txs_processed: 0,
            tx_failures: 0,
            token_update_reports: 0,
            created_tokens_unique: 0,
            updated_tokens_unique: 0,
            discovered_v2_pools_unique: 0,
            updated_v2_pools_unique: 0,
            discovered_v3_pools_unique: 0,
            updated_v3_pools_unique: 0,
            discovered_v4_pools_unique: 0,
            updated_v4_pools_unique: 0,
            tracked_tokens: 0,
            indexed_tokens: 0,
            indexed_pools: 0,
            tracked_pools: 0,
            indexed_v2_pools: 0,
            tracked_v2_pools: 0,
            indexed_v3_pools: 0,
            tracked_v3_pools: 0,
            indexed_v4_pools: 0,
            tracked_v4_pools: 0,
            retention_evaluated_tokens: 0,
            retention_dropped_tokens: 0,
            retention_dropped_v2_pools: 0,
            processed_block_disk_cache_hits: 0,
            processed_block_disk_cache_misses: 0,
            live_stream_events: 0,
            live_gap_blocks_caught_up: 0,
            last_stream_id: None,
            last_block_source: None,
            last_block_upstream_ms: None,
            last_block_token_apply_ms: None,
            last_block_disk_cache_read_ms: None,
            last_block_disk_cache_write_ms: None,
            started_at_unix_secs: Some(now),
            live_at_unix_secs: None,
            updated_at_unix_secs: now,
            completed_at_unix_secs: None,
            last_error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LiveTokenProgress, LiveTokenStatus};

    #[test]
    fn live_status_is_busy() {
        assert!(LiveTokenStatus::Live.is_busy());
        assert!(LiveTokenStatus::Warming.is_busy());
        assert!(!LiveTokenStatus::Idle.is_busy());
    }

    #[test]
    fn warming_progress_records_range() {
        let progress = LiveTokenProgress::warming("live-1", 100, 10, 12, 1);
        assert_eq!(progress.status, LiveTokenStatus::Warming);
        assert_eq!(progress.warmup_total_blocks, 3);
        assert_eq!(progress.live_at_unix_secs, None);
    }
}
