use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use eth_ops_events::{PipelineBottleneckSample, PipelineIssue};
use eth_pool_classification::PoolCohort;
use eth_token::erc20::{ERC20Token, TokenSummary};
use eth_token::token_analytics::TokenPoolCurrentObservation;
use eth_token::tracking::{LiveTokenRetentionPolicy, LiveTokenRetentionReport, TrackedTokenStatus};
use serde::Serialize;

use crate::live::{LiveTracker, LiveTrackerError, LiveTrackerProgress};
use crate::read_models::surface::{self, PoolSurfaceFilter, TokenPoolSurfaceResponse};
use crate::read_models::token::{TokenPnlView, TokenView};
use crate::read_models::{pool::PoolView, token_analytics::TokenNetworkView};
use crate::recent_blocks::{RecentLiveBlocks, RecentProcessedBlock};

#[derive(Clone, Debug, Serialize)]
pub struct LiveStatusResponse {
    pub progress: LiveTrackerProgress,
    pub summary: LiveProgressSummary,
    pub errors: Vec<LiveTrackerError>,
    pub issues: Vec<PipelineIssue>,
    pub bottlenecks: Vec<PipelineBottleneckSample>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveTokenListResponse {
    pub progress: LiveTrackerProgress,
    pub summary: LiveProgressSummary,
    pub count: usize,
    pub tokens: Vec<TokenView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveTokenDetailResponse {
    pub progress: LiveTrackerProgress,
    pub progress_summary: LiveProgressSummary,
    pub token: ERC20Token,
    pub summary: TokenSummary,
    pub index_status: Option<TrackedTokenStatus>,
    pub pools: Vec<PoolView>,
    pub observations: Vec<TokenPoolCurrentObservation>,
    pub network: TokenNetworkView,
    pub pnl: TokenPnlView,
    pub denom_symbols: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LivePoolListResponse {
    pub progress: LiveTrackerProgress,
    pub summary: LiveProgressSummary,
    pub filter: PoolSurfaceFilter,
    pub count: usize,
    pub total_count: usize,
    pub active_count: usize,
    pub non_scam_count: usize,
    pub eligible_active_count: usize,
    pub scam_count: usize,
    pub eligible_count: usize,
    pub ineligible_count: usize,
    pub ineligible_reason_counts: Vec<LivePoolReasonCount>,
    pub pools: Vec<PoolView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveProgressSummary {
    pub total_blocks_processed: u64,
    pub warmup_blocks_processed: u64,
    pub warmup_blocks_remaining: u64,
    pub warmup_progress_percent: Option<f64>,
    pub live_blocks_processed: u64,
    pub blocks_since_live_started: u64,
    pub live_start_block: Option<u64>,
    pub run_duration_secs: Option<u64>,
    pub warmup_duration_secs: Option<u64>,
    pub live_duration_secs: Option<u64>,
    pub seconds_since_last_update: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct LivePoolReasonCount {
    pub key: String,
    pub label: String,
    pub count: usize,
    pub share_percent: Option<f64>,
    pub ineligible_share_percent: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveRetentionResponse {
    pub progress: LiveTrackerProgress,
    pub summary: LiveProgressSummary,
    pub policy: Option<LiveTokenRetentionPolicy>,
    pub last_report: Option<LiveTokenRetentionReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecentProcessedBlocksResponse {
    pub count: usize,
    pub blocks: Vec<RecentProcessedBlock>,
}

pub async fn status(tracker: &LiveTracker) -> LiveStatusResponse {
    let state = tracker.state().await;
    let progress = state.progress.clone();
    LiveStatusResponse {
        summary: live_progress_summary(&progress),
        progress,
        errors: state.errors.clone(),
        issues: state.issues.clone(),
        bottlenecks: state.bottlenecks.clone(),
    }
}

pub async fn token_list(tracker: &LiveTracker) -> LiveTokenListResponse {
    let state = tracker.state().await;
    let mut tokens = Vec::with_capacity(state.processor.registry().tokens.len());

    for token in state.processor.registry().tokens.values() {
        let index_status = index_status(&state, &token.contract_address);
        tokens.push(TokenView::from_token(token, index_status));
    }

    tokens.sort_by(|left, right| {
        right
            .creation_timestamp
            .cmp(&left.creation_timestamp)
            .then(right.creation_block.cmp(&left.creation_block))
            .then(left.contract_address.cmp(&right.contract_address))
    });

    let progress = state.progress.clone();
    LiveTokenListResponse {
        summary: live_progress_summary(&progress),
        progress,
        count: tokens.len(),
        tokens,
    }
}

pub async fn surface(tracker: &LiveTracker) -> TokenPoolSurfaceResponse {
    let state = tracker.state().await;
    let progress = state.progress.clone();
    surface::build_token_pool_surface(
        surface::TokenPoolSurfaceContext {
            surface: surface::TokenPoolSurfaceKind::Live,
            run_id: progress.id.clone(),
        },
        Some(live_progress_summary(&progress)),
        state.processor.registry().tokens.values(),
        |token_address| index_status(&state, token_address),
    )
}

pub async fn token_detail(
    tracker: &LiveTracker,
    token_address: &str,
) -> Option<LiveTokenDetailResponse> {
    let state = tracker.state().await;
    let address = normalize_address(token_address);
    let token = state.processor.registry().tokens.get(&address)?;
    let index_status = index_status(&state, &address);
    let network = TokenNetworkView::from_graph(
        token,
        state
            .processor
            .block_processor()
            .network_graphs
            .get(&address),
    );
    let recent_activity = token.activity.recent_blocks(50);
    let pools = PoolView::from_token_pools_with_activity(token, &recent_activity);
    let summary = crate::read_models::token::token_summary_with_pool_views(token, &pools);
    let denom_symbols = crate::read_models::token::build_denom_symbols(token);
    let pnl = TokenPnlView::from_token(token);
    let observations = crate::read_models::token::token_observations_with_backfill(
        &state.observations,
        &address,
        token,
    );

    let progress = state.progress.clone();
    Some(LiveTokenDetailResponse {
        progress_summary: live_progress_summary(&progress),
        progress,
        token: token.clone(),
        summary,
        index_status,
        pools,
        observations,
        network,
        pnl,
        denom_symbols,
    })
}

pub async fn pool_list(tracker: &LiveTracker, filter: PoolSurfaceFilter) -> LivePoolListResponse {
    let state = tracker.state().await;
    let mut pools = Vec::new();

    for token in state.processor.registry().tokens.values() {
        pools.extend(PoolView::from_token_pool_summaries(token));
    }

    let total_count = pools.len();
    let scam_count = pools.iter().filter(|pool| pool.is_scam).count();
    let non_scam_count = total_count.saturating_sub(scam_count);
    let eligible_count = pools
        .iter()
        .filter(|pool| pool.pool_classification.cohort == PoolCohort::Eligible)
        .count();
    let eligible_active_count = pools
        .iter()
        .filter(|pool| pool.pool_classification.eligible && !pool.is_scam)
        .count();
    let active_count = eligible_active_count;
    let ineligible_count = total_count.saturating_sub(eligible_count);
    let ineligible_reason_counts = ineligible_reason_counts(&pools, total_count, ineligible_count);
    pools.retain(|pool| filter.matches(pool));

    pools.sort_by(|left, right| {
        left.token_address
            .cmp(&right.token_address)
            .then(left.pool_address.cmp(&right.pool_address))
    });

    let progress = state.progress.clone();
    LivePoolListResponse {
        summary: live_progress_summary(&progress),
        progress,
        filter,
        count: pools.len(),
        total_count,
        active_count,
        non_scam_count,
        eligible_active_count,
        scam_count,
        eligible_count,
        ineligible_count,
        ineligible_reason_counts,
        pools,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_progress_summary_separates_warmup_and_live_blocks() {
        let mut progress = LiveTrackerProgress::warming("run", 500, 100, 199, 1_000);
        progress.blocks_processed = 125;
        progress.live_blocks_processed = 25;
        progress.live_at_unix_secs = Some(1_080);
        progress.updated_at_unix_secs = 1_090;

        let summary = LiveProgressSummary::from_progress_at(&progress, 1_120);

        assert_eq!(summary.total_blocks_processed, 125);
        assert_eq!(summary.warmup_blocks_processed, 100);
        assert_eq!(summary.warmup_blocks_remaining, 0);
        assert_eq!(summary.live_blocks_processed, 25);
        assert_eq!(summary.blocks_since_live_started, 25);
        assert_eq!(summary.live_start_block, Some(200));
        assert_eq!(summary.run_duration_secs, Some(120));
        assert_eq!(summary.warmup_duration_secs, Some(80));
        assert_eq!(summary.live_duration_secs, Some(40));
        assert_eq!(summary.seconds_since_last_update, 30);
    }
}

pub async fn retention(tracker: &LiveTracker) -> LiveRetentionResponse {
    let state = tracker.state().await;
    let progress = state.progress.clone();
    LiveRetentionResponse {
        summary: live_progress_summary(&progress),
        progress,
        policy: state
            .processor
            .block_processor()
            .token_index
            .live_retention_policy()
            .cloned(),
        last_report: state.last_retention_report.clone(),
    }
}

pub fn recent_processed_blocks(
    recent_live_blocks: &RecentLiveBlocks,
    limit: usize,
) -> RecentProcessedBlocksResponse {
    let blocks = recent_live_blocks.latest(limit.max(1));
    RecentProcessedBlocksResponse {
        count: blocks.len(),
        blocks,
    }
}

pub fn live_progress_summary(progress: &LiveTrackerProgress) -> LiveProgressSummary {
    LiveProgressSummary::from_progress_at(progress, now_unix_secs())
}

impl LiveProgressSummary {
    fn from_progress_at(progress: &LiveTrackerProgress, now: u64) -> Self {
        let warmup_blocks_processed = progress
            .blocks_processed
            .saturating_sub(progress.live_blocks_processed)
            .min(progress.warmup_total_blocks);
        let warmup_blocks_remaining = progress
            .warmup_total_blocks
            .saturating_sub(warmup_blocks_processed);
        let completed_or_now = progress.completed_at_unix_secs.unwrap_or(now);
        let live_start_block = progress
            .warmup_end_block
            .and_then(|block| block.checked_add(1));

        Self {
            total_blocks_processed: progress.blocks_processed,
            warmup_blocks_processed,
            warmup_blocks_remaining,
            warmup_progress_percent: percent(
                warmup_blocks_processed as usize,
                progress.warmup_total_blocks as usize,
            ),
            live_blocks_processed: progress.live_blocks_processed,
            blocks_since_live_started: progress.live_blocks_processed,
            live_start_block,
            run_duration_secs: progress
                .started_at_unix_secs
                .map(|started| completed_or_now.saturating_sub(started)),
            warmup_duration_secs: progress.started_at_unix_secs.map(|started| {
                progress
                    .live_at_unix_secs
                    .unwrap_or(completed_or_now)
                    .saturating_sub(started)
            }),
            live_duration_secs: progress
                .live_at_unix_secs
                .map(|live_at| completed_or_now.saturating_sub(live_at)),
            seconds_since_last_update: now.saturating_sub(progress.updated_at_unix_secs),
        }
    }
}

fn ineligible_reason_counts(
    pools: &[PoolView],
    total_count: usize,
    ineligible_count: usize,
) -> Vec<LivePoolReasonCount> {
    let mut counts = BTreeMap::<(String, String), usize>::new();
    for pool in pools {
        if pool.pool_classification.cohort != PoolCohort::Ineligible {
            continue;
        }
        let key = pool
            .pool_classification
            .reason_key
            .unwrap_or("unknown")
            .to_string();
        let label = pool
            .pool_classification
            .reason_label
            .unwrap_or("Unknown")
            .to_string();
        *counts.entry((key, label)).or_default() += 1;
    }

    let mut rows: Vec<_> = counts
        .into_iter()
        .map(|((key, label), count)| LivePoolReasonCount {
            key,
            label,
            count,
            share_percent: percent(count, total_count),
            ineligible_share_percent: percent(count, ineligible_count),
        })
        .collect();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then(left.label.cmp(&right.label))
    });
    rows
}

fn percent(count: usize, total: usize) -> Option<f64> {
    (total > 0).then(|| (count as f64 / total as f64) * 100.0)
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn index_status(
    state: &crate::live::LiveTrackerState,
    token_address: &str,
) -> Option<TrackedTokenStatus> {
    state
        .processor
        .block_processor()
        .token_index
        .entries
        .get(&normalize_address(token_address))
        .map(|entry| entry.token_status.clone())
}

fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
