use std::convert::Infallible;

use serde::Serialize;
use serde_json::json;
use warp::http::StatusCode;
use warp::{Filter, Reply};

use crate::error::ApiError;
use crate::live::StartLiveTrackerRequest;
use crate::mempool_signals::{MempoolSignalKind, MempoolSignalQuery};
use crate::range_indexer::StartRangeIndexRequest;
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

    let processed_block_disk_cache_coverage = warp::path!("cache" / "coverage")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(processed_block_disk_cache_coverage);

    let live_status = warp::path!("live" / "status")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live_status);

    let live_start = warp::path!("live" / "start")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(live_start);

    let live_stop = warp::path!("live" / "stop")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(live_stop);

    let live_tokens = warp::path!("live" / "tokens")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live_tokens);

    let live_pools = warp::path!("live" / "pools")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live_pools);

    let live_token_detail = warp::path!("live" / "tokens" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live_token_detail);

    let live_retention = warp::path!("live" / "retention")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live_retention);

    let mempool_signals = warp::path!("mempool" / "signals")
        .and(warp::get())
        .and(warp::query::<MempoolSignalQuery>())
        .and(with_state(state.clone()))
        .and_then(mempool_signals);

    let mempool_signals_by_type = warp::path!("mempool" / "signals" / String)
        .and(warp::get())
        .and(warp::query::<MempoolSignalQuery>())
        .and(with_state(state.clone()))
        .and_then(mempool_signals_by_type);

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
        .or(processed_block_disk_cache_coverage)
        .or(live_status)
        .or(live_start)
        .or(live_stop)
        .or(live_token_detail)
        .or(live_tokens)
        .or(live_pools)
        .or(live_retention)
        .or(mempool_signals_by_type)
        .or(mempool_signals)
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
            "default_blocks": state.config.default_blocks,
            "live_warmup_blocks": state.config.live_warmup_blocks,
            "processed_block_disk_cache_dir": state.config.processed_block_disk_cache_dir,
            "processed_block_disk_cache_blocks": state.config.processed_block_disk_cache_blocks,
            "redis_url": state.config.redis_url,
            "live_block_stream": state.config.live_block_stream,
            "live_processed_block_disk_cache_retry_attempts": state.config.live_processed_block_disk_cache_retry_attempts,
            "live_processed_block_disk_cache_retry_delay_ms": state.config.live_processed_block_disk_cache_retry_delay_ms,
            "live_stream_block_ms": state.config.live_stream_block_ms,
            "live_stream_count": state.config.live_stream_count,
            "live_block_apply_timeout_ms": state.config.live_block_apply_timeout_ms,
            "mempool_signal_limit": state.config.mempool_signal_limit,
        }),
        StatusCode::OK,
    ))
}

async fn live_status(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::status(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

async fn live_start(
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

async fn live_stop(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    state.live_tracker.stop().await;
    Ok(json_response(
        &views::live::status(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

async fn live_tokens(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::token_list(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

async fn live_pools(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::pool_list(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

async fn live_token_detail(
    token_address: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match views::live::token_detail(&state.live_tracker, &token_address).await {
        Some(detail) => Ok(json_response(&detail, StatusCode::OK)),
        None => Ok(error_response("token not found", StatusCode::NOT_FOUND)),
    }
}

async fn live_retention(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &views::live::retention(&state.live_tracker).await,
        StatusCode::OK,
    ))
}

async fn mempool_signals(
    query: MempoolSignalQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state
        .mempool_signals
        .list(MempoolSignalKind::All, query)
        .await
    {
        Ok(signals) => Ok(json_response(&signals, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load mempool signals: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

async fn mempool_signals_by_type(
    signal_type: String,
    query: MempoolSignalQuery,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    let Some(kind) = MempoolSignalKind::from_path(&signal_type) else {
        return Ok(error_response("unknown signal type", StatusCode::NOT_FOUND));
    };

    match state.mempool_signals.list(kind, query).await {
        Ok(signals) => Ok(json_response(&signals, StatusCode::OK)),
        Err(error) => Ok(error_response(
            format!("failed to load mempool signals: {error}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

async fn list_runs(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    let runs = state.range_indexer.list_runs().await;
    Ok(json_response(
        &views::run::RunListResponse { runs },
        StatusCode::OK,
    ))
}

async fn start_run(
    request: StartRangeIndexRequest,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.start_run(request).await {
        Ok(run) => {
            let progress = views::run::progress(&run).await;
            Ok(json_response(&progress, StatusCode::CREATED))
        }
        Err(error) => Ok(error_response(error.to_string(), StatusCode::BAD_REQUEST)),
    }
}

async fn processed_block_disk_cache_coverage(
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

async fn run_progress(
    run_id: String,
    state: ServerState,
) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.get_run(&run_id).await {
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
    match state.range_indexer.get_run(&run_id).await {
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
    match state.range_indexer.get_run(&run_id).await {
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
    match state.range_indexer.get_run(&run_id).await {
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
    match state.range_indexer.get_run(&run_id).await {
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
    match state.range_indexer.get_run(&run_id).await {
        Some(run) => Ok(warp::sse::reply(
            warp::sse::keep_alive().stream(sse::progress_stream(run)),
        )
        .into_response()),
        None => Ok(error_response("run not found", StatusCode::NOT_FOUND)),
    }
}

async fn stop_run(run_id: String, state: ServerState) -> Result<warp::reply::Response, Infallible> {
    match state.range_indexer.stop_run(&run_id).await {
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
