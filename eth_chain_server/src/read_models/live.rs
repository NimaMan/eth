use std::collections::BTreeMap;

use eth_ops_events::{PipelineBottleneckSample, PipelineIssue};
use eth_token::erc20::{ERC20Token, TokenSummary};
use eth_token::tracking::{LiveTokenRetentionPolicy, LiveTokenRetentionReport, TrackedTokenStatus};
use serde::{Deserialize, Serialize};

use crate::live::{LiveTracker, LiveTrackerError, LiveTrackerProgress};
use crate::read_models::token::TokenView;
use crate::read_models::{network::TokenNetworkView, pool::PoolView};
use crate::recent_blocks::{RecentLiveBlocks, RecentProcessedBlock};

#[derive(Clone, Debug, Serialize)]
pub struct LiveStatusResponse {
    pub progress: LiveTrackerProgress,
    pub errors: Vec<LiveTrackerError>,
    pub issues: Vec<PipelineIssue>,
    pub bottlenecks: Vec<PipelineBottleneckSample>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveTokenListResponse {
    pub progress: LiveTrackerProgress,
    pub count: usize,
    pub tokens: Vec<TokenView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveTokenDetailResponse {
    pub progress: LiveTrackerProgress,
    pub token: ERC20Token,
    pub summary: TokenSummary,
    pub index_status: Option<TrackedTokenStatus>,
    pub pools: Vec<PoolView>,
    pub network: TokenNetworkView,
    pub denom_symbols: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LivePoolListResponse {
    pub progress: LiveTrackerProgress,
    pub filter: LivePoolStatusFilter,
    pub count: usize,
    pub total_count: usize,
    pub active_count: usize,
    pub scam_count: usize,
    pub pools: Vec<PoolView>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivePoolStatusFilter {
    All,
    Active,
    Scam,
}

impl Default for LivePoolStatusFilter {
    fn default() -> Self {
        Self::All
    }
}

impl LivePoolStatusFilter {
    fn matches_scam_flag(self, is_scam: bool) -> bool {
        match self {
            Self::All => true,
            Self::Active => !is_scam,
            Self::Scam => is_scam,
        }
    }

    fn matches(self, pool: &PoolView) -> bool {
        self.matches_scam_flag(pool.is_scam)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveRetentionResponse {
    pub progress: LiveTrackerProgress,
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
    LiveStatusResponse {
        progress: state.progress.clone(),
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

    LiveTokenListResponse {
        progress: state.progress.clone(),
        count: tokens.len(),
        tokens,
    }
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

    Some(LiveTokenDetailResponse {
        progress: state.progress.clone(),
        token: token.clone(),
        summary,
        index_status,
        pools,
        network,
        denom_symbols,
    })
}

pub async fn pool_list(
    tracker: &LiveTracker,
    filter: LivePoolStatusFilter,
) -> LivePoolListResponse {
    let state = tracker.state().await;
    let mut pools = Vec::new();

    for token in state.processor.registry().tokens.values() {
        pools.extend(PoolView::from_token_pool_summaries(token));
    }

    let total_count = pools.len();
    let scam_count = pools.iter().filter(|pool| pool.is_scam).count();
    let active_count = total_count.saturating_sub(scam_count);
    pools.retain(|pool| filter.matches(pool));

    pools.sort_by(|left, right| {
        left.token_address
            .cmp(&right.token_address)
            .then(left.pool_address.cmp(&right.pool_address))
    });

    LivePoolListResponse {
        progress: state.progress.clone(),
        filter,
        count: pools.len(),
        total_count,
        active_count,
        scam_count,
        pools,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_status_filter_matches_expected_pool_sets() {
        assert!(LivePoolStatusFilter::All.matches_scam_flag(false));
        assert!(LivePoolStatusFilter::All.matches_scam_flag(true));
        assert!(LivePoolStatusFilter::Active.matches_scam_flag(false));
        assert!(!LivePoolStatusFilter::Active.matches_scam_flag(true));
        assert!(!LivePoolStatusFilter::Scam.matches_scam_flag(false));
        assert!(LivePoolStatusFilter::Scam.matches_scam_flag(true));
    }
}

pub async fn retention(tracker: &LiveTracker) -> LiveRetentionResponse {
    let state = tracker.state().await;
    LiveRetentionResponse {
        progress: state.progress.clone(),
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
