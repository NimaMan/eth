use std::convert::Infallible;

use warp::http::StatusCode;
use warp::Reply;

use crate::http::reply::{error_response, json_response};
use crate::http::sse;
use crate::http::ServerState;
use crate::ranges::{StartRangeIndexError, StartRangeIndexRequest};
use crate::read_models as views;

#[derive(Debug, serde::Deserialize)]
pub(super) struct RangePoolsQuery {
    #[serde(default)]
    status: views::surface::PoolSurfaceFilter,
}

pub(super) async fn list_runs(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    let runs = state.range_indexer.list_runs().await;
    Ok(json_response(
        &views::run::RunListResponse { runs },
        StatusCode::OK,
    ))
}

pub(super) async fn start_run(
    request: StartRangeIndexRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.start_run(request).await {
        Ok(run) => {
            let progress = views::run::progress_response(&run).await;
            Ok(json_response(&progress, StatusCode::CREATED))
        }
        Err(StartRangeIndexError::ActiveRunConflict { active_run_id }) => Ok(error_response(
            format!("active run already exists: {active_run_id}"),
            StatusCode::CONFLICT,
        )),
        Err(StartRangeIndexError::InvalidRequest(error)) => {
            Ok(error_response(error.to_string(), StatusCode::BAD_REQUEST))
        }
    }
}

pub(super) async fn active_run(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.active_run().await {
        Some(run) => Ok(json_response(
            &views::run::progress_response(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response(
            "active run not found",
            StatusCode::NOT_FOUND,
        )),
    }
}

pub(super) async fn stop_active_run(
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.stop_active_run().await {
        Some(run) => Ok(json_response(
            &views::run::progress_response(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response(
            "active run not found",
            StatusCode::NOT_FOUND,
        )),
    }
}

pub(super) async fn active_run_launch_stats(
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.active_run().await {
        Some(run) => Ok(json_response(
            &views::strategy::launch_stats(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response(
            "active run not found",
            StatusCode::NOT_FOUND,
        )),
    }
}

pub(super) async fn processed_block_disk_cache_coverage(
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match views::cache::coverage(
        state.processed_block_disk_cache.as_deref(),
        state
            .config
            .processed_block_disk_cache_dir
            .as_ref()
            .map(|_| state.config.processed_block_disk_cache_blocks),
    ) {
        Ok(coverage) => Ok(json_response(&coverage, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to inspect processed block disk cache: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub(super) async fn progress(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.get_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::run::progress_response(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

pub(super) async fn tokens(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.get_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::token::token_list(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

pub(super) async fn token_detail(
    run_id: String,
    token_address: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.get_run(&run_id).await {
        Some(run) => match views::token::token_detail(&run, &token_address).await {
            Some(detail) => Ok(json_response(&detail, StatusCode::OK)),
            None => Ok(error_response("token not found", StatusCode::NOT_FOUND)),
        },
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

pub(super) async fn pools(
    run_id: String,
    query: RangePoolsQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.get_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::pool::pool_list_filtered(&run, query.status).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

pub(super) async fn surface(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.get_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::surface::range_surface(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

pub(super) async fn launch_stats(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.get_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::strategy::launch_stats(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

pub(super) async fn errors(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.get_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::error::error_list(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

pub(super) async fn stream(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.get_run(&run_id).await {
        Some(run) => Ok(warp::sse::reply(
            warp::sse::keep_alive().stream(sse::progress_stream(run)),
        )
        .into_response()),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

pub(super) async fn stop_run(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.stop_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::run::progress_response(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}
