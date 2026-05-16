use std::sync::Arc;

use serde::Serialize;

use crate::ranges::{RangeIndexJob, RangeIndexProgress, RangeIndexRetentionMode, RangeIndexStatus};

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

#[derive(Clone, Debug, Serialize)]
pub struct RunProgressResponse {
    #[serde(flatten)]
    pub progress: RangeIndexProgress,
    pub summary: RunProgressSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunProgressSummary {
    pub total_blocks_processed: u64,
    pub blocks_remaining: u64,
    pub progress_percent: Option<f64>,
    pub run_duration_secs: u64,
    pub seconds_since_last_update: u64,
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

pub async fn progress_response(run: &RangeIndexJob) -> RunProgressResponse {
    let progress = run.progress().await;
    let summary = RunProgressSummary::from_progress_at(&progress, now_unix_secs());
    RunProgressResponse { progress, summary }
}

impl RunProgressSummary {
    fn from_progress_at(progress: &RangeIndexProgress, now: u64) -> Self {
        let completed_or_now = progress.completed_at_unix_secs.unwrap_or(now);
        Self {
            total_blocks_processed: progress.blocks_processed,
            blocks_remaining: progress
                .total_blocks
                .saturating_sub(progress.blocks_processed),
            progress_percent: percent(
                progress.blocks_processed as usize,
                progress.total_blocks as usize,
            ),
            run_duration_secs: completed_or_now.saturating_sub(progress.started_at_unix_secs),
            seconds_since_last_update: now.saturating_sub(progress.updated_at_unix_secs),
        }
    }
}

fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn percent(count: usize, total: usize) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some(count as f64 / total as f64 * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranges::RangeIndexProgress;

    #[test]
    fn range_progress_summary_uses_backend_time_fields() {
        let mut progress = RangeIndexProgress::new("run-1", 10, 19, 100);
        progress.blocks_processed = 4;
        progress.updated_at_unix_secs = 130;

        let summary = RunProgressSummary::from_progress_at(&progress, 160);

        assert_eq!(summary.total_blocks_processed, 4);
        assert_eq!(summary.blocks_remaining, 6);
        assert_eq!(summary.progress_percent, Some(40.0));
        assert_eq!(summary.run_duration_secs, 60);
        assert_eq!(summary.seconds_since_last_update, 30);
    }
}
