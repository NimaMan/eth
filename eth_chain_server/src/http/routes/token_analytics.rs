use std::convert::Infallible;

use serde_json::json;
use warp::http::StatusCode;

use crate::http::reply::{error_response, json_response};
use crate::http::ServerState;
use crate::read_models as views;
use crate::token_analytics::network::{
    StartTokenNetworkAnalysisError, TokenNetworkAnalysisRequest,
};

pub(super) async fn start(
    request: TokenNetworkAnalysisRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.token_network_analytics.start_analysis(request).await {
        Ok(job) => Ok(json_response(
            &views::token_analytics::job(&job).await,
            StatusCode::CREATED,
        )),
        Err(StartTokenNetworkAnalysisError::InvalidRequest(error)) => {
            Ok(error_response(error.to_string(), StatusCode::BAD_REQUEST))
        }
        Err(StartTokenNetworkAnalysisError::Spawn(error)) => Ok(error_response(
            format!("failed to spawn token network analysis job: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn list(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::token_analytics::list(state.token_network_analytics.list_jobs().await).await,
        StatusCode::OK,
    ))
}

pub(super) async fn risk_atlas(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    match state.risk_atlas.latest_page_view().await {
        Ok(Some(view)) => Ok(json_response(&view, StatusCode::OK)),
        Ok(None) => Ok(error_response(
            "risk atlas has no imported snapshot",
            StatusCode::NOT_FOUND,
        )),
        Err(error) => Ok(error_response(
            format!("failed to load risk atlas: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn risk_atlas_runs(
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.risk_atlas.runs(200).await {
        Ok(runs) => Ok(json_response(&json!({ "runs": runs }), StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to list risk atlas runs: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn risk_atlas_run(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.risk_atlas.page_view(&run_id).await {
        Ok(Some(view)) => Ok(json_response(&view, StatusCode::OK)),
        Ok(None) => Ok(error_response(
            "risk atlas run not found",
            StatusCode::NOT_FOUND,
        )),
        Err(error) => Ok(error_response(
            format!("failed to load risk atlas run: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn scammer_analytics(
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let mut payload = views::scammer_analytics::payload();
    if let Some(object) = payload.as_object_mut() {
        match state.risk_atlas.scammer_address_distribution(250).await {
            Ok(Some(distribution)) => {
                object.insert("address_distribution".to_string(), distribution);
            }
            Ok(None) => {
                object.insert(
                    "address_distribution".to_string(),
                    json!({
                        "summary": {},
                        "buckets": [],
                        "top_addresses": [],
                        "error": "no token PnL run found"
                    }),
                );
            }
            Err(error) => {
                object.insert(
                    "address_distribution".to_string(),
                    json!({
                        "summary": {},
                        "buckets": [],
                        "top_addresses": [],
                        "error": format!("failed to load token PnL address distribution: {error}")
                    }),
                );
            }
        }
    }
    Ok(json_response(&payload, StatusCode::OK))
}

pub(super) async fn get(
    job_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.token_network_analytics.get_job(&job_id).await {
        Some(job) => Ok(json_response(
            &views::token_analytics::job(&job).await,
            StatusCode::OK,
        )),
        None => Ok(error_response(
            "token network analysis job not found",
            StatusCode::NOT_FOUND,
        )),
    }
}

pub(super) async fn cancel(
    job_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.token_network_analytics.cancel_job(&job_id).await {
        Some(job) => Ok(json_response(
            &views::token_analytics::job(&job).await,
            StatusCode::OK,
        )),
        None => Ok(error_response(
            "token network analysis job not found",
            StatusCode::NOT_FOUND,
        )),
    }
}
