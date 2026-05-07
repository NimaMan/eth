use std::sync::Arc;

use serde::Serialize;

use crate::historical::{RunProgress, RunStatus, TrackingRun};

#[derive(Clone, Debug, Serialize)]
pub struct RunListResponse {
    pub runs: Vec<RunSummaryView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunSummaryView {
    pub id: String,
    pub status: RunStatus,
    pub start_block: u64,
    pub end_block: u64,
    pub total_blocks: u64,
    pub blocks_processed: u64,
    pub tracked_tokens: usize,
    pub indexed_v2_pools: usize,
    pub tx_failures: usize,
    pub started_at_unix_secs: u64,
    pub updated_at_unix_secs: u64,
    pub completed_at_unix_secs: Option<u64>,
}

impl RunSummaryView {
    pub async fn from_run(run: &Arc<TrackingRun>) -> Self {
        let progress = run.progress().await;
        Self {
            id: progress.id,
            status: progress.status,
            start_block: progress.start_block,
            end_block: progress.end_block,
            total_blocks: progress.total_blocks,
            blocks_processed: progress.blocks_processed,
            tracked_tokens: progress.tracked_tokens,
            indexed_v2_pools: progress.indexed_v2_pools,
            tx_failures: progress.tx_failures,
            started_at_unix_secs: progress.started_at_unix_secs,
            updated_at_unix_secs: progress.updated_at_unix_secs,
            completed_at_unix_secs: progress.completed_at_unix_secs,
        }
    }
}

pub async fn progress(run: &TrackingRun) -> RunProgress {
    run.progress().await
}
