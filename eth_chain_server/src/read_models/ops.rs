use std::collections::BTreeMap;

use eth_ops_events::{
    emit_health, PipelineBottleneckSample, PipelineHealth, PipelineHealthStatus, PipelineImpact,
    PipelineIssue,
};
use serde::Serialize;
use serde_json::json;

use crate::live::{LiveTracker, LiveTrackerStatus};

#[derive(Clone, Debug, Serialize)]
pub struct OpsHealthResponse {
    pub count: usize,
    pub health: Vec<PipelineHealth>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OpsIssueListResponse {
    pub count: usize,
    pub groups: Vec<PipelineIssueGroup>,
    pub issues: Vec<PipelineIssue>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PipelineIssueGroup {
    pub dedupe_key: String,
    pub count: usize,
    pub latest: PipelineIssue,
    pub first_seen_block: Option<u64>,
    pub last_seen_block: Option<u64>,
    pub fatal: bool,
    pub impact: PipelineImpact,
}

#[derive(Clone, Debug, Serialize)]
pub struct OpsBottleneckListResponse {
    pub count: usize,
    pub bottlenecks: Vec<PipelineBottleneckSample>,
}

pub async fn health(tracker: &LiveTracker) -> OpsHealthResponse {
    let state = tracker.state().await;
    let progress = &state.progress;
    let status = match progress.status {
        LiveTrackerStatus::Live => {
            if state.issues.iter().any(|issue| issue.fatal) {
                PipelineHealthStatus::Degraded
            } else {
                PipelineHealthStatus::Healthy
            }
        }
        LiveTrackerStatus::Warming | LiveTrackerStatus::Stopping => PipelineHealthStatus::Watch,
        LiveTrackerStatus::Failed => PipelineHealthStatus::Down,
        LiveTrackerStatus::Idle | LiveTrackerStatus::Stopped => PipelineHealthStatus::Unknown,
    };
    let mut live = PipelineHealth::new(
        "eth_chain_server",
        "live_tracker",
        "live_token_runtime",
        status,
    );
    live.run_id = progress.id.clone();
    live.current_block = progress.current_block;
    live.metrics.insert(
        "blocks_processed".to_string(),
        json!(progress.blocks_processed),
    );
    live.metrics.insert(
        "live_blocks_processed".to_string(),
        json!(progress.live_blocks_processed),
    );
    live.metrics
        .insert("tx_failures".to_string(), json!(progress.tx_failures));
    live.metrics
        .insert("tracked_tokens".to_string(), json!(progress.tracked_tokens));
    live.metrics
        .insert("tracked_pools".to_string(), json!(progress.tracked_pools));
    live.metrics.insert(
        "last_block_token_apply_ms".to_string(),
        json!(progress.last_block_token_apply_ms),
    );
    live.dependencies.insert(
        "processed_block_cache".to_string(),
        if progress.processed_block_disk_cache_misses > 0 {
            PipelineHealthStatus::Watch
        } else {
            PipelineHealthStatus::Healthy
        },
    );
    live.dependencies.insert(
        "direct_live_block_loop".to_string(),
        if progress.status == LiveTrackerStatus::Live || progress.live_blocks_processed > 0 {
            PipelineHealthStatus::Healthy
        } else {
            PipelineHealthStatus::Unknown
        },
    );

    emit_health(&live);
    OpsHealthResponse {
        count: 1,
        health: vec![live],
    }
}

pub async fn issues(tracker: &LiveTracker) -> OpsIssueListResponse {
    let state = tracker.state().await;
    let mut issues = state.issues.clone();
    issues.sort_by(|left, right| {
        right
            .ts_unix_ms
            .cmp(&left.ts_unix_ms)
            .then(right.block_number.cmp(&left.block_number))
            .then(right.tx_index.cmp(&left.tx_index))
    });
    let groups = group_issues(&issues);
    OpsIssueListResponse {
        count: issues.len(),
        groups,
        issues,
    }
}

pub async fn bottlenecks(tracker: &LiveTracker) -> OpsBottleneckListResponse {
    let state = tracker.state().await;
    let mut bottlenecks = state.bottlenecks.clone();
    bottlenecks.sort_by(|left, right| {
        right
            .ts_unix_ms
            .cmp(&left.ts_unix_ms)
            .then(right.duration_ms.cmp(&left.duration_ms))
    });
    OpsBottleneckListResponse {
        count: bottlenecks.len(),
        bottlenecks,
    }
}

fn group_issues(issues: &[PipelineIssue]) -> Vec<PipelineIssueGroup> {
    let mut grouped: BTreeMap<String, Vec<&PipelineIssue>> = BTreeMap::new();
    for issue in issues {
        grouped
            .entry(issue.dedupe_key.clone())
            .or_default()
            .push(issue);
    }

    let mut groups = grouped
        .into_iter()
        .filter_map(|(dedupe_key, rows)| {
            let latest = rows
                .iter()
                .max_by(|left, right| left.ts_unix_ms.cmp(&right.ts_unix_ms))?;
            let first_seen_block = rows.iter().filter_map(|issue| issue.block_number).min();
            let last_seen_block = rows.iter().filter_map(|issue| issue.block_number).max();
            Some(PipelineIssueGroup {
                dedupe_key,
                count: rows.len(),
                latest: (*latest).clone(),
                first_seen_block,
                last_seen_block,
                fatal: rows.iter().any(|issue| issue.fatal),
                impact: latest.impact,
            })
        })
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        right
            .fatal
            .cmp(&left.fatal)
            .then(right.count.cmp(&left.count))
            .then(right.latest.ts_unix_ms.cmp(&left.latest.ts_unix_ms))
    });
    groups
}
