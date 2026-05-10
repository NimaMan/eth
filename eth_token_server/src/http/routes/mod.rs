mod alpha;
mod health;
mod live;
mod mempool;
mod range;
mod token_activity;

use std::convert::Infallible;

use warp::{Filter, Reply};

use crate::http::ServerState;
use crate::read_models::activity::TokenActivityBlocksQuery;
use crate::stores::alpha_trading::StrategyPerformanceQuery;
use crate::stores::mempool_signals::MempoolSignalQuery;

pub fn routes(
    state: ServerState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(["content-type"])
        .allow_methods(["GET", "POST", "OPTIONS"]);

    api(state).or(crate::http::assets::static_routes()).with(cors)
}

fn api(state: ServerState) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let health = warp::path!("health")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(health::health);

    let list_runs = warp::path!("runs")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::list_runs);

    let start_run = warp::path!("runs")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(range::start_run);

    let active_run = warp::path!("runs" / "active")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::active_run);

    let stop_active_run = warp::path!("runs" / "active" / "stop")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(range::stop_active_run);

    let active_run_launch_stats = warp::path!("runs" / "active" / "strategy" / "launch-stats")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::active_run_launch_stats);

    let processed_block_disk_cache_coverage = warp::path!("cache" / "coverage")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::processed_block_disk_cache_coverage);

    let live_status = warp::path!("live" / "status")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::status);

    let live_start = warp::path!("live" / "start")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(live::start);

    let live_stop = warp::path!("live" / "stop")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(live::stop);

    let live_tokens = warp::path!("live" / "tokens")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::tokens);

    let live_pools = warp::path!("live" / "pools")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::pools);

    let live_token_detail = warp::path!("live" / "tokens" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::token_detail);

    let live_retention = warp::path!("live" / "retention")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::retention);

    let mempool_signals = warp::path!("mempool" / "signals")
        .and(warp::get())
        .and(warp::query::<MempoolSignalQuery>())
        .and(with_state(state.clone()))
        .and_then(mempool::signals);

    let mempool_signals_by_type = warp::path!("mempool" / "signals" / String)
        .and(warp::get())
        .and(warp::query::<MempoolSignalQuery>())
        .and(with_state(state.clone()))
        .and_then(mempool::signals_by_type);

    let token_activity_blocks = warp::path!("tokens" / String / "activity-blocks")
        .and(warp::get())
        .and(warp::query::<TokenActivityBlocksQuery>())
        .and(with_state(state.clone()))
        .and_then(token_activity::activity_blocks);

    let alpha_strategies = warp::path!("alpha" / "strategies")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(alpha::strategies);

    let alpha_strategy_detail = warp::path!("alpha" / "strategies" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(alpha::strategy_detail);

    let alpha_strategy_performance = warp::path!("alpha" / "strategies" / String / "performance")
        .and(warp::get())
        .and(warp::query::<StrategyPerformanceQuery>())
        .and(with_state(state.clone()))
        .and_then(alpha::strategy_performance);

    let alpha_strategy_reset = warp::path!("alpha" / "strategies" / String / "reset-paper-state")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(alpha::strategy_reset);

    let progress = warp::path!("runs" / String / "progress")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::progress);

    let tokens = warp::path!("runs" / String / "tokens")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::tokens);

    let token_detail = warp::path!("runs" / String / "tokens" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::token_detail);

    let pools = warp::path!("runs" / String / "pools")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::pools);

    let launch_stats = warp::path!("runs" / String / "strategy" / "launch-stats")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::launch_stats);

    let errors = warp::path!("runs" / String / "errors")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::errors);

    let stream = warp::path!("runs" / String / "stream")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::stream);

    let stop = warp::path!("runs" / String / "stop")
        .and(warp::post())
        .and(with_state(state))
        .and_then(range::stop_run);

    health
        .or(list_runs)
        .or(start_run)
        .or(active_run)
        .or(stop_active_run)
        .or(active_run_launch_stats)
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
        .or(token_activity_blocks)
        .or(alpha_strategy_performance)
        .or(alpha_strategy_reset)
        .or(alpha_strategy_detail)
        .or(alpha_strategies)
        .or(token_detail)
        .or(tokens)
        .or(progress)
        .or(pools)
        .or(launch_stats)
        .or(errors)
        .or(stream)
        .or(stop)
}

fn with_state(
    state: ServerState,
) -> impl Filter<Extract = (ServerState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}
