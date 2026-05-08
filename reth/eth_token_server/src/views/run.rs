use std::sync::Arc;

use serde::Serialize;

use crate::range_indexer::{
    RangeIndexJob, RangeIndexProgress, RangeIndexRetentionMode, RangeIndexStatus,
};

#[derive(Clone, Debug, Serialize)]
pub struct RunListResponse {
    pub runs: Vec<RunSummaryView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunSummaryView {
    pub id: String,
    pub status: RangeIndexStatus,
    pub start_block: u64,
    pub end_block: u64,
    pub total_blocks: u64,
    pub retention_mode: RangeIndexRetentionMode,
    pub blocks_processed: u64,
    pub tracked_tokens: usize,
    pub indexed_pools: usize,
    pub indexed_v2_pools: usize,
    pub indexed_v3_pools: usize,
    pub indexed_v4_pools: usize,
    pub tx_failures: usize,
    pub started_at_unix_secs: u64,
    pub updated_at_unix_secs: u64,
    pub completed_at_unix_secs: Option<u64>,
}

impl RunSummaryView {
    pub async fn from_run(run: &Arc<RangeIndexJob>) -> Self {
        let progress = run.progress().await;
        Self {
            id: progress.id,
            status: progress.status,
            start_block: progress.start_block,
            end_block: progress.end_block,
            total_blocks: progress.total_blocks,
            retention_mode: run.request.retention_mode,
            blocks_processed: progress.blocks_processed,
            tracked_tokens: progress.tracked_tokens,
            indexed_pools: progress.indexed_pools,
            indexed_v2_pools: progress.indexed_v2_pools,
            indexed_v3_pools: progress.indexed_v3_pools,
            indexed_v4_pools: progress.indexed_v4_pools,
            tx_failures: progress.tx_failures,
            started_at_unix_secs: progress.started_at_unix_secs,
            updated_at_unix_secs: progress.updated_at_unix_secs,
            completed_at_unix_secs: progress.completed_at_unix_secs,
        }
    }
}

pub async fn progress(run: &RangeIndexJob) -> RangeIndexProgress {
    run.progress().await
}
