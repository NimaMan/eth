use std::convert::Infallible;

use warp::http::StatusCode;

use crate::http::reply::json_response;
use crate::http::ServerState;
use crate::read_models::ops as ops_views;

pub(super) async fn health(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &ops_views::health(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

pub(super) async fn issues(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &ops_views::issues(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

pub(super) async fn bottlenecks(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &ops_views::bottlenecks(&state.live_tracker).await,
        StatusCode::OK,
    ))
}
