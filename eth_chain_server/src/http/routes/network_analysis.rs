use std::convert::Infallible;

use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;
use crate::network_analysis::{NetworkAnalysisRequest, StartNetworkAnalysisError};
use crate::read_models as views;

pub(super) async fn start(
    request: NetworkAnalysisRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.network_analysis.start_analysis(request).await {
        Ok(job) => Ok(json_response(
            &views::network_analysis::job(&job).await,
            StatusCode::CREATED,
        )),
        Err(StartNetworkAnalysisError::InvalidRequest(error)) => {
            Ok(error_response(error.to_string(), StatusCode::BAD_REQUEST))
        }
        Err(StartNetworkAnalysisError::Spawn(error)) => Ok(error_response(
            format!("failed to spawn network analysis job: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn list(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::network_analysis::list(state.network_analysis.list_jobs().await).await,
        StatusCode::OK,
    ))
}

pub(super) async fn get(
    job_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.network_analysis.get_job(&job_id).await {
        Some(job) => Ok(json_response(
            &views::network_analysis::job(&job).await,
            StatusCode::OK,
        )),
        None => Ok(error_response(
            "network analysis job not found",
            StatusCode::NOT_FOUND,
        )),
    }
}

pub(super) async fn cancel(
    job_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.network_analysis.cancel_job(&job_id).await {
        Some(job) => Ok(json_response(
            &views::network_analysis::job(&job).await,
            StatusCode::OK,
        )),
        None => Ok(error_response(
            "network analysis job not found",
            StatusCode::NOT_FOUND,
        )),
    }
}
