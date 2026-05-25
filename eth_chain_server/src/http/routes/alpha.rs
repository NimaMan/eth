use std::convert::Infallible;

use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;
use crate::read_models::gas_rank::{self, GasRankEstimateRequest, GasRankSamplesRequest};
use crate::stores::alpha_trading::{AlphaStrategyResetRequest, StrategyPerformanceQuery};

pub(super) async fn strategies(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    match state.alpha_trading.list_strategies().await {
        Ok(strategies) => Ok(json_response(&strategies, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load alpha strategies: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn strategy_detail(
    strategy_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.alpha_trading.strategy_detail(&strategy_id).await {
        Ok(Some(strategy)) => Ok(json_response(&strategy, StatusCode::OK)),
        Ok(None) => Ok(error_response(
            "alpha strategy not found",
            StatusCode::NOT_FOUND,
        )),
        Err(error) => Ok(error_response(
            format!("failed to load alpha strategy: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn strategy_performance(
    strategy_id: String,
    query: StrategyPerformanceQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state
        .alpha_trading
        .strategy_performance(&strategy_id, query)
        .await
    {
        Ok(Some(performance)) => Ok(json_response(&performance, StatusCode::OK)),
        Ok(None) => Ok(error_response(
            "alpha strategy not found",
            StatusCode::NOT_FOUND,
        )),
        Err(error) => Ok(error_response(
            format!("failed to load alpha strategy performance: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn strategy_reset(
    strategy_id: String,
    request: AlphaStrategyResetRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state
        .alpha_trading
        .reset_strategy_state(&strategy_id, request)
        .await
    {
        Ok(Some(reset)) => Ok(json_response(&reset, StatusCode::OK)),
        Ok(None) => Ok(error_response(
            "alpha strategy not found",
            StatusCode::NOT_FOUND,
        )),
        Err(error) => Ok(error_response(
            format!("failed to reset alpha strategy state: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn gas_rank_estimate(
    request: GasRankEstimateRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match gas_rank::estimate(&state, request).await {
        Ok(estimate) => Ok(json_response(&estimate, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to estimate gas rank: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn gas_rank_samples(
    query: GasRankSamplesRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let limit = query.limit.unwrap_or(100);
    Ok(json_response(
        &gas_rank::recent_samples(&state, limit),
        StatusCode::OK,
    ))
}
