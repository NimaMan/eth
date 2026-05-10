use std::convert::Infallible;

use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;
use crate::live::StartLiveTrackerRequest;
use crate::read_models as views;

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
    match state.live_tracker.start(request).await {
        Ok(_) => Ok(json_response(
            &views::live::status(&state.live_tracker).await,
            StatusCode::CREATED,
        )),
        Err(error) => Ok(error_response(error.to_string(), StatusCode::BAD_REQUEST)),
    }
}

pub(super) async fn stop(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    state.live_tracker.stop().await;
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

pub(super) async fn pools(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::pool_list(&state.live_tracker).await,
        StatusCode::OK,
    ))
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
