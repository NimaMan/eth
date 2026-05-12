use std::collections::BTreeMap;

use eth_pipeline_telemetry::{PipelineBottleneckSample, PipelineIssue};
use eth_token::erc20::{ERC20Token, TokenSummary};
use eth_token::tracking::{LiveTokenRetentionPolicy, LiveTokenRetentionReport, TrackedTokenStatus};
use serde::Serialize;

use crate::live::{LiveTracker, LiveTrackerError, LiveTrackerProgress};
use crate::read_models::token::{TokenActivitySummary, TokenView};
use crate::read_models::{network::TokenNetworkView, pool::PoolView};

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
    pub activity_summary: TokenActivitySummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct LivePoolListResponse {
    pub progress: LiveTrackerProgress,
    pub count: usize,
    pub pools: Vec<PoolView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LiveRetentionResponse {
    pub progress: LiveTrackerProgress,
    pub policy: Option<LiveTokenRetentionPolicy>,
    pub last_report: Option<LiveTokenRetentionReport>,
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
    let denom_symbols = crate::read_models::token::build_denom_symbols(token);
    let activity_summary = crate::read_models::token::build_activity_summary(token);

    Some(LiveTokenDetailResponse {
        progress: state.progress.clone(),
        token: token.clone(),
        summary: token.get_token_summary(),
        index_status,
        pools,
        network,
        denom_symbols,
        activity_summary,
    })
}

pub async fn pool_list(tracker: &LiveTracker) -> LivePoolListResponse {
    let state = tracker.state().await;
    let mut pools = Vec::new();

    for token in state.processor.registry().tokens.values() {
        pools.extend(PoolView::from_token_pool_summaries(token));
    }

    pools.sort_by(|left, right| {
        left.token_address
            .cmp(&right.token_address)
            .then(left.pool_address.cmp(&right.pool_address))
    });

    LivePoolListResponse {
        progress: state.progress.clone(),
        count: pools.len(),
        pools,
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
