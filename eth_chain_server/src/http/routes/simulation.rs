use std::convert::Infallible;

use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;
use crate::live_simulation::{
    self as live_simulation_service, LiveOrderSimulationRequest, LivePoolBuySellSimulationRequest,
    LiveUnsignedTxSequenceSimulationRequest, LiveUnsignedTxSimulationRequest,
};
use crate::read_models::simulation::{self, VaultSimulationRequest};

pub(super) async fn vault_uniswap_v2(
    request: VaultSimulationRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match simulation::simulate_uniswap_v2_trading_vault_wallet_token(
        state.provider.simulator().as_ref(),
        request,
    )
    .await
    {
        Ok(report) => Ok(json_response(&report, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("vault simulation failed: {error}"),
            simulation_error_status(&error),
        )),
    }
}

pub(super) async fn live_unsigned_tx(
    request: LiveUnsignedTxSimulationRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match live_simulation_service::simulate_live_unsigned_transaction(&state.live_tracker, request)
        .await
    {
        Ok(report) => Ok(json_response(&report, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("live unsigned tx simulation failed: {error}"),
            simulation_error_status(&error),
        )),
    }
}

pub(super) async fn live_unsigned_tx_sequence(
    request: LiveUnsignedTxSequenceSimulationRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match live_simulation_service::simulate_live_unsigned_transaction_sequence(
        &state.live_tracker,
        request,
    )
    .await
    {
        Ok(report) => Ok(json_response(&report, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("live unsigned tx sequence simulation failed: {error}"),
            simulation_error_status(&error),
        )),
    }
}

pub(super) async fn live_pool_buy_sell(
    request: LivePoolBuySellSimulationRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match live_simulation_service::simulate_live_pool_buy_sell(&state.live_tracker, request).await {
        Ok(report) => Ok(json_response(&report, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("live pool buy/sell simulation failed: {error}"),
            simulation_error_status(&error),
        )),
    }
}

pub(super) async fn live_order(
    request: LiveOrderSimulationRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match live_simulation_service::simulate_live_order(&state.live_tracker, request).await {
        Ok(report) => Ok(json_response(&report, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("live order simulation failed: {error}"),
            simulation_error_status(&error),
        )),
    }
}

fn simulation_error_status(error: &eyre::Report) -> StatusCode {
    let message = error.to_string();
    if message.contains("exact live simulation state is unavailable")
        || message.contains("in-memory live block state")
    {
        return StatusCode::CONFLICT;
    }
    if message.contains("invalid ")
        || message.contains("below requested transfer amount")
        || message.contains("requires ")
        || message.contains("must")
    {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
