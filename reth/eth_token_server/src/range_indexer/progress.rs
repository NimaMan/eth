use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RangeIndexStatus {
    Queued,
    Running,
    Completed,
    Stopping,
    Stopped,
    Failed,
}

impl RangeIndexStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Stopped | Self::Failed)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RangeIndexProgress {
    pub id: String,
    pub status: RangeIndexStatus,
    pub start_block: u64,
    pub end_block: u64,
    pub total_blocks: u64,
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
    pub last_block_upstream_ms: Option<u128>,
    pub last_block_token_apply_ms: Option<u128>,
    pub processed_block_disk_cache_hits: u64,
    pub processed_block_disk_cache_misses: u64,
    pub last_block_disk_cache_read_ms: Option<u128>,
    pub last_block_disk_cache_write_ms: Option<u128>,
    pub last_block_source: Option<String>,
    pub started_at_unix_secs: u64,
    pub updated_at_unix_secs: u64,
    pub completed_at_unix_secs: Option<u64>,
    pub last_error: Option<String>,
}

impl RangeIndexProgress {
    pub fn new(
        id: impl Into<String>,
        start_block: u64,
        end_block: u64,
        started_at_unix_secs: u64,
    ) -> Self {
        Self {
            id: id.into(),
            status: RangeIndexStatus::Queued,
            start_block,
            end_block,
            total_blocks: end_block - start_block + 1,
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
            last_block_upstream_ms: None,
            last_block_token_apply_ms: None,
            processed_block_disk_cache_hits: 0,
            processed_block_disk_cache_misses: 0,
            last_block_disk_cache_read_ms: None,
            last_block_disk_cache_write_ms: None,
            last_block_source: None,
            started_at_unix_secs,
            updated_at_unix_secs: started_at_unix_secs,
            completed_at_unix_secs: None,
            last_error: None,
        }
    }
}

pub fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
