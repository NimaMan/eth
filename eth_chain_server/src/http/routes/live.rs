use std::convert::Infallible;
use std::time::Duration;

use eth_live_feed::{LiveTokenEvent, LiveTokenReader, LiveTokenStatus};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;
use crate::live::StartLiveTrackerRequest;
use crate::live_simulation::LiveTxSimulatorStatusResponse;
use crate::read_models as views;

const DEFAULT_UPDATE_WAIT_MS: u64 = 30_000;
const MAX_UPDATE_WAIT_MS: u64 = 120_000;
const DEFAULT_RECENT_BLOCK_LIMIT: usize = 5;
const MAX_RECENT_BLOCK_LIMIT: usize = 100;

#[derive(Debug, Deserialize)]
pub(super) struct LiveUpdatesQuery {
    #[serde(default)]
    after_block: Option<u64>,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RecentProcessedBlocksQuery {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LivePoolsQuery {
    #[serde(default)]
    status: views::surface::PoolSurfaceFilter,
}

#[derive(Debug, Serialize)]
struct LiveUpdatesResponse {
    event: &'static str,
    status: String,
    block_number: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    updated_tokens: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    updated_v2_pools: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    updated_v3_pools: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    updated_v4_pools: Vec<String>,
}

pub(super) async fn status(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::status(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

pub(super) async fn start(
    request: StartLiveTrackerRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.live_chain_runtime.start(request).await {
        Ok(_) => Ok(json_response(
            &views::live::status(&state.live_tracker).await,
            StatusCode::CREATED,
        )),
        Err(error) => Ok(error_response(error.to_string(), StatusCode::BAD_REQUEST)),
    }
}

pub(super) async fn stop(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    state.live_chain_runtime.stop().await;
    Ok(json_response(
        &views::live::status(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

pub(super) async fn tokens(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::token_list(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

pub(super) async fn surface(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::surface(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

pub(super) async fn pools(
    query: LivePoolsQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::pool_list(&state.live_tracker, query.status).await,
        StatusCode::OK,
    ))
}

pub(super) async fn active_pools(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::pool_list(
            &state.live_tracker,
            views::surface::PoolSurfaceFilter::Active,
        )
        .await,
        StatusCode::OK,
    ))
}

pub(super) async fn scam_pools(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::pool_list(&state.live_tracker, views::surface::PoolSurfaceFilter::Scam).await,
        StatusCode::OK,
    ))
}

pub(super) async fn eligible_pools(
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::pool_list(
            &state.live_tracker,
            views::surface::PoolSurfaceFilter::Eligible,
        )
        .await,
        StatusCode::OK,
    ))
}

pub(super) async fn ineligible_pools(
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::pool_list(
            &state.live_tracker,
            views::surface::PoolSurfaceFilter::Ineligible,
        )
        .await,
        StatusCode::OK,
    ))
}

pub(super) async fn updates(
    query: LiveUpdatesQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let after_block = query.after_block.unwrap_or_default();
    let wait_ms = query
        .timeout_ms
        .unwrap_or(DEFAULT_UPDATE_WAIT_MS)
        .clamp(1_000, MAX_UPDATE_WAIT_MS);
    let initial_progress = state.live_tracker.progress().await;
    if initial_progress.current_block.unwrap_or_default() > after_block {
        return Ok(json_response(
            &LiveUpdatesResponse::from_progress("already_ahead", &initial_progress),
            StatusCode::OK,
        ));
    }

    let mut updates = state.live_tracker.subscribe();
    let tracker = state.live_tracker.clone();
    let response = match tokio::time::timeout(Duration::from_millis(wait_ms), async move {
        loop {
            match updates.recv().await {
                Ok(event) => {
                    if let Some(response) = event_response(event, after_block, &tracker).await {
                        return response;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    let progress = tracker.progress().await;
                    if progress.current_block.unwrap_or_default() > after_block {
                        return LiveUpdatesResponse::from_progress("lag_refresh", &progress);
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    let progress = tracker.progress().await;
                    return LiveUpdatesResponse::from_progress("closed", &progress);
                }
            }
        }
    })
    .await
    {
        Ok(response) => response,
        Err(_) => {
            let progress = state.live_tracker.progress().await;
            LiveUpdatesResponse::from_progress("timeout", &progress)
        }
    };

    Ok(json_response(&response, StatusCode::OK))
}

pub(super) async fn token_detail(
    token_address: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match views::live::token_detail(&state.live_tracker, &token_address).await {
        Some(detail) => Ok(json_response(&detail, StatusCode::OK)),
        None => Ok(error_response("token not found", StatusCode::NOT_FOUND)),
    }
}

pub(super) async fn retention(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::retention(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

pub(super) async fn processed_blocks(
    query: RecentProcessedBlocksQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_RECENT_BLOCK_LIMIT)
        .clamp(1, MAX_RECENT_BLOCK_LIMIT);
    Ok(json_response(
        &views::live::recent_processed_blocks(&state.recent_live_blocks, limit),
        StatusCode::OK,
    ))
}

pub(super) async fn latest_state_frame(
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let response = views::live::latest_live_state_frame(&state.recent_live_state_frames);
    let status = if response.available {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    Ok(json_response(&response, status))
}

pub(super) async fn live_tx_simulator_status(
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state
        .live_tracker
        .live_tx_simulator()
        .latest_state_status()
        .await
    {
        Ok(status) => {
            let base_fee_per_gas_wei = state
                .recent_live_state_frames
                .latest()
                .filter(|frame| frame.frame.header.number == status.selected_block_number)
                .and_then(|frame| frame.frame.header.base_fee_per_gas)
                .map(|fee| fee.to_string());
            Ok(json_response(
                &LiveTxSimulatorStatusResponse {
                    schema: "eth_live_tx_simulator_status_v1",
                    available: true,
                    unavailable_reason: None,
                    selected_block_number: Some(status.selected_block_number),
                    selected_block_hash: status.selected_block_hash,
                    state_source: "chain_server_live_tx_simulator",
                    latest_reth_finished_block_number: Some(
                        status.latest_reth_finished_block_number,
                    ),
                    latest_historical_context_block_number: Some(
                        status.latest_historical_context_block_number,
                    ),
                    latest_live_block_number: status.latest_live_block_number,
                    latest_tracked_state_block_number: status.latest_tracked_state_block_number,
                    base_fee_per_gas_wei,
                },
                StatusCode::OK,
            ))
        }
        Err(error) => Ok(json_response(
            &LiveTxSimulatorStatusResponse {
                schema: "eth_live_tx_simulator_status_v1",
                available: false,
                unavailable_reason: Some(error.to_string()),
                selected_block_number: None,
                selected_block_hash: None,
                state_source: "chain_server_live_tx_simulator",
                latest_reth_finished_block_number: None,
                latest_historical_context_block_number: None,
                latest_live_block_number: None,
                latest_tracked_state_block_number: None,
                base_fee_per_gas_wei: None,
            },
            StatusCode::SERVICE_UNAVAILABLE,
        )),
    }
}

async fn event_response(
    event: LiveTokenEvent,
    after_block: u64,
    tracker: &crate::live::LiveTracker,
) -> Option<LiveUpdatesResponse> {
    match event {
        LiveTokenEvent::BlockApplied {
            block_number,
            updated_tokens,
            updated_v2_pools,
            updated_v3_pools,
            updated_v4_pools,
            ..
        } if block_number > after_block => {
            let progress = tracker.progress().await;
            Some(LiveUpdatesResponse {
                event: "block_applied",
                status: status_label(&progress.status).to_string(),
                block_number: Some(block_number),
                updated_tokens,
                updated_v2_pools,
                updated_v3_pools,
                updated_v4_pools,
            })
        }
        LiveTokenEvent::RuntimeLive { current_block, .. }
            if current_block.unwrap_or_default() > after_block =>
        {
            let progress = tracker.progress().await;
            Some(LiveUpdatesResponse::from_progress(
                "runtime_live",
                &progress,
            ))
        }
        LiveTokenEvent::RuntimeFailed { block_number, .. } => {
            let progress = tracker.progress().await;
            Some(LiveUpdatesResponse {
                event: "runtime_failed",
                status: status_label(&progress.status).to_string(),
                block_number,
                updated_tokens: Vec::new(),
                updated_v2_pools: Vec::new(),
                updated_v3_pools: Vec::new(),
                updated_v4_pools: Vec::new(),
            })
        }
        LiveTokenEvent::RuntimeStopped { current_block, .. } => {
            let progress = tracker.progress().await;
            Some(LiveUpdatesResponse {
                event: "runtime_stopped",
                status: status_label(&progress.status).to_string(),
                block_number: current_block,
                updated_tokens: Vec::new(),
                updated_v2_pools: Vec::new(),
                updated_v3_pools: Vec::new(),
                updated_v4_pools: Vec::new(),
            })
        }
        _ => None,
    }
}

impl LiveUpdatesResponse {
    fn from_progress(event: &'static str, progress: &eth_live_feed::LiveTokenProgress) -> Self {
        Self {
            event,
            status: status_label(&progress.status).to_string(),
            block_number: progress.current_block,
            updated_tokens: Vec::new(),
            updated_v2_pools: Vec::new(),
            updated_v3_pools: Vec::new(),
            updated_v4_pools: Vec::new(),
        }
    }
}

fn status_label(status: &LiveTokenStatus) -> &'static str {
    match status {
        LiveTokenStatus::Idle => "idle",
        LiveTokenStatus::Warming => "warming",
        LiveTokenStatus::Live => "live",
        LiveTokenStatus::Stopping => "stopping",
        LiveTokenStatus::Stopped => "stopped",
        LiveTokenStatus::Failed => "failed",
    }
}
