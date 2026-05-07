use eth_token::erc20::{ERC20Token, TokenSummary};
use eth_token::manager::{LiveTokenRetentionPolicy, LiveTokenRetentionReport, TrackedTokenStatus};
use serde::Serialize;

use crate::live::{LiveTracker, LiveTrackerError, LiveTrackerProgress};
use crate::views::pool::PoolView;
use crate::views::token::TokenView;

#[derive(Clone, Debug, Serialize)]
pub struct LiveStatusResponse {
    pub progress: LiveTrackerProgress,
    pub errors: Vec<LiveTrackerError>,
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
    let mut pools = token
        .v2_pools
        .values()
        .map(|pool| PoolView::from_pool(token, pool))
        .collect::<Vec<_>>();
    pools.sort_by(|left, right| left.pool_address.cmp(&right.pool_address));

    Some(LiveTokenDetailResponse {
        progress: state.progress.clone(),
        token: token.clone(),
        summary: token.get_token_summary(),
        index_status,
        pools,
    })
}

pub async fn pool_list(tracker: &LiveTracker) -> LivePoolListResponse {
    let state = tracker.state().await;
    let mut pools = Vec::new();

    for token in state.processor.registry().tokens.values() {
        for pool in token.v2_pools.values() {
            pools.push(PoolView::from_pool(token, pool));
        }
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
