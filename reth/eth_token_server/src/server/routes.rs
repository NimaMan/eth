use std::convert::Infallible;

use serde::Serialize;
use serde_json::json;
use warp::http::StatusCode;
use warp::{Filter, Reply};

use crate::error::ApiError;
use crate::runs::StartRunRequest;
use crate::server::sse;
use crate::server::ServerState;
use crate::views;

pub fn routes(
    state: ServerState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(["content-type"])
        .allow_methods(["GET", "POST", "OPTIONS"]);

    api(state).with(cors)
}

fn api(state: ServerState) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let health = warp::path!("health")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(health);

    let list_runs = warp::path!("runs")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(list_runs);

    let start_run = warp::path!("runs")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(start_run);

    let progress = warp::path!("runs" / String / "progress")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(run_progress);

    let tokens = warp::path!("runs" / String / "tokens")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(run_tokens);

    let token_detail = warp::path!("runs" / String / "tokens" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(run_token_detail);

    let pools = warp::path!("runs" / String / "pools")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(run_pools);

    let errors = warp::path!("runs" / String / "errors")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(run_errors);

    let stream = warp::path!("runs" / String / "stream")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(run_stream);

    let stop = warp::path!("runs" / String / "stop")
        .and(warp::post())
        .and(with_state(state))
        .and_then(stop_run);

    health
        .or(list_runs)
        .or(start_run)
        .or(token_detail)
        .or(tokens)
        .or(progress)
        .or(pools)
        .or(errors)
        .or(stream)
        .or(stop)
}

fn with_state(
    state: ServerState,
) -> impl Filter<Extract = (ServerState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn health(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &json!({
            "status": "ok",
            "bind": state.config.bind.to_string(),
            "reth_datadir": state.config.reth_datadir,
            "history_limit": state.config.history_limit,
            "max_blocks": state.config.max_blocks,
            "default_blocks": state.config.default_blocks,
            "processed_block_cache_dir": state.config.processed_block_cache_dir,
            "processed_block_cache_blocks": state.config.processed_block_cache_blocks,
        }),
        StatusCode::OK,
    ))
}

async fn list_runs(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    let runs = state.runs.list_runs().await;
    Ok(json_response(
        &views::run::RunListResponse { runs },
        StatusCode::OK,
    ))
}

async fn start_run(
    request: StartRunRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.runs.start_run(request).await {
        Ok(run) => {
            let progress = views::run::progress(&run).await;
            Ok(json_response(&progress, StatusCode::CREATED))
        }
        Err(error) => Ok(error_response(error.to_string(), StatusCode::BAD_REQUEST)),
    }
}

async fn run_progress(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.runs.get_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::run::progress(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

async fn run_tokens(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.runs.get_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::token::token_list(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

async fn run_token_detail(
    run_id: String,
    token_address: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.runs.get_run(&run_id).await {
        Some(run) => match views::token::token_detail(&run, &token_address).await {
            Some(detail) => Ok(json_response(&detail, StatusCode::OK)),
            None => Ok(error_response("token not found", StatusCode::NOT_FOUND)),
        },
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

async fn run_pools(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.runs.get_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::pool::pool_list(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

async fn run_errors(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.runs.get_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::error::error_list(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

async fn run_stream(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.runs.get_run(&run_id).await {
        Some(run) => Ok(warp::sse::reply(
            warp::sse::keep_alive().stream(sse::progress_stream(run)),
        )
        .into_response()),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

async fn stop_run(run_id: String, state: ServerState) -> Result<warp::reply::Response, Infallible> {
    match state.runs.stop_run(&run_id).await {
        Some(run) => Ok(json_response(
            &views::run::progress(&run).await,
            StatusCode::OK,
        )),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

fn json_response<T: Serialize>(value: &T, status: StatusCode) -> warp::reply::Response {
    warp::reply::with_status(warp::reply::json(value), status).into_response()
}

fn error_response(message: impl Into<String>, status: StatusCode) -> warp::reply::Response {
    json_response(&ApiError::new(message), status)
}
