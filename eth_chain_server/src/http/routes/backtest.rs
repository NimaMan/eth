use std::convert::Infallible;

use serde::Deserialize;
use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;
use crate::stores::alpha_trading::{
    ResultSetDetailQuery, ResultSetListQuery, ResultSetPerformanceQuery,
};

#[derive(Debug, Deserialize)]
pub struct RunListQuery {
    pub mode: Option<String>,
    pub limit: Option<i64>,
}

pub(super) async fn list_runs(
    query: RunListQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let limit = query.limit.unwrap_or(50);
    let mode = query.mode.as_deref();
    match state.alpha_trading.list_runs(mode, limit).await {
        Ok(runs) => Ok(json_response(&runs, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to list runs: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn run_detail(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.alpha_trading.run_detail(&run_id).await {
        Ok(Some(run)) => Ok(json_response(&run, StatusCode::OK)),
        Ok(None) => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
        Err(error) => Ok(error_response(
            format!("failed to load run: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn result_sets(
    query: ResultSetListQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.alpha_trading.result_sets(query).await {
        Ok(result_sets) => Ok(json_response(&result_sets, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to list result sets: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn result_set_detail(
    result_set_id: String,
    query: ResultSetDetailQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state
        .alpha_trading
        .result_set_detail(&result_set_id, query)
        .await
    {
        Ok(Some(result_set)) => Ok(json_response(&result_set, StatusCode::OK)),
        Ok(None) => Ok(error_response(
            "result set not found",
            StatusCode::NOT_FOUND,
        )),
        Err(error) => Ok(error_response(
            format!("failed to load result set: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn result_set_performance(
    result_set_id: String,
    query: ResultSetPerformanceQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state
        .alpha_trading
        .result_set_performance(&result_set_id, query)
        .await
    {
        Ok(performance) => Ok(json_response(&performance, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load result set performance: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn result_set_strategy_performance(
    result_set_id: String,
    strategy_name: String,
    mut query: ResultSetPerformanceQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    query.strategy_name = Some(strategy_name);
    result_set_performance(result_set_id, query, state).await
}

pub(super) async fn run_positions(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.alpha_trading.run_positions(&run_id, 100).await {
        Ok(positions) => Ok(json_response(&positions, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load run positions: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn run_orders(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.alpha_trading.run_orders(&run_id, 100).await {
        Ok(orders) => Ok(json_response(&orders, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load run orders: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn run_execution_reports(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state
        .alpha_trading
        .run_execution_reports(&run_id, 250)
        .await
    {
        Ok(reports) => Ok(json_response(&reports, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load run execution reports: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn run_risk_events(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.alpha_trading.run_risk_events(&run_id, 100).await {
        Ok(events) => Ok(json_response(&events, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load run risk events: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn run_strategy_decisions(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state
        .alpha_trading
        .run_strategy_decisions(&run_id, 100)
        .await
    {
        Ok(decisions) => Ok(json_response(&decisions, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load run strategy decisions: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn run_position_decision_audit(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state
        .alpha_trading
        .run_position_decision_audit(&run_id, 10)
        .await
    {
        Ok(rows) => Ok(json_response(&rows, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load run position decision audit: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}
