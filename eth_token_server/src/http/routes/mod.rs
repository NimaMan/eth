mod alpha;
mod backtest;
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

    api(state).with(cors)
}

fn api(state: ServerState) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let health = warp::path!("eth" / "tokens" / "api" / "health")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(health::health);

    let list_runs = warp::path!("eth" / "tokens" / "api" / "runs")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::list_runs);

    let start_run = warp::path!("eth" / "tokens" / "api" / "runs")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(range::start_run);

    let active_run = warp::path!("eth" / "tokens" / "api" / "runs" / "active")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::active_run);

    let stop_active_run = warp::path!("eth" / "tokens" / "api" / "runs" / "active" / "stop")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(range::stop_active_run);

    let active_run_launch_stats =
        warp::path!("eth" / "tokens" / "api" / "runs" / "active" / "strategy" / "launch-stats")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(range::active_run_launch_stats);

    let processed_block_disk_cache_coverage =
        warp::path!("eth" / "tokens" / "api" / "cache" / "coverage")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(range::processed_block_disk_cache_coverage);

    let live_status = warp::path!("eth" / "tokens" / "api" / "live" / "status")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::status);

    let live_start = warp::path!("eth" / "tokens" / "api" / "live" / "start")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(live::start);

    let live_stop = warp::path!("eth" / "tokens" / "api" / "live" / "stop")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(live::stop);

    let live_tokens = warp::path!("eth" / "tokens" / "api" / "live" / "tokens")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::tokens);

    let live_pools = warp::path!("eth" / "tokens" / "api" / "live" / "pools")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::pools);

    let live_token_detail = warp::path!("eth" / "tokens" / "api" / "live" / "tokens" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::token_detail);

    let live_retention = warp::path!("eth" / "tokens" / "api" / "live" / "retention")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::retention);

    let mempool_signals = warp::path!("eth" / "tokens" / "api" / "mempool" / "signals")
        .and(warp::get())
        .and(warp::query::<MempoolSignalQuery>())
        .and(with_state(state.clone()))
        .and_then(mempool::signals);

    let mempool_signals_by_type =
        warp::path!("eth" / "tokens" / "api" / "mempool" / "signals" / String)
            .and(warp::get())
            .and(warp::query::<MempoolSignalQuery>())
            .and(with_state(state.clone()))
            .and_then(mempool::signals_by_type);

    let token_activity_blocks =
        warp::path!("eth" / "tokens" / "api" / "tokens" / String / "activity-blocks")
            .and(warp::get())
            .and(warp::query::<TokenActivityBlocksQuery>())
            .and(with_state(state.clone()))
            .and_then(token_activity::activity_blocks);

    let alpha_strategies = warp::path!("eth" / "tokens" / "api" / "alpha" / "strategies")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(alpha::strategies);

    let alpha_strategy_detail =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "strategies" / String)
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(alpha::strategy_detail);

    let alpha_strategy_performance =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "strategies" / String / "performance")
            .and(warp::get())
            .and(warp::query::<StrategyPerformanceQuery>())
            .and(with_state(state.clone()))
            .and_then(alpha::strategy_performance);

    let alpha_strategy_reset =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "strategies" / String / "reset-state")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_state(state.clone()))
            .and_then(alpha::strategy_reset);

    let alpha_runs = warp::path!("eth" / "tokens" / "api" / "alpha" / "runs")
        .and(warp::get())
        .and(warp::query::<backtest::RunListQuery>())
        .and(with_state(state.clone()))
        .and_then(backtest::list_runs);

    let alpha_run_detail = warp::path!("eth" / "tokens" / "api" / "alpha" / "runs" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(backtest::run_detail);

    let alpha_run_positions =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "runs" / String / "positions")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(backtest::run_positions);

    let alpha_run_orders =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "runs" / String / "orders")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(backtest::run_orders);

    let alpha_run_reports =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "runs" / String / "reports")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(backtest::run_execution_reports);

    let alpha_run_risks =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "runs" / String / "risks")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(backtest::run_risk_events);

    let progress = warp::path!("eth" / "tokens" / "api" / "runs" / String / "progress")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::progress);

    let tokens = warp::path!("eth" / "tokens" / "api" / "runs" / String / "tokens")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::tokens);

    let token_detail = warp::path!("eth" / "tokens" / "api" / "runs" / String / "tokens" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::token_detail);

    let pools = warp::path!("eth" / "tokens" / "api" / "runs" / String / "pools")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::pools);

    let launch_stats =
        warp::path!("eth" / "tokens" / "api" / "runs" / String / "strategy" / "launch-stats")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(range::launch_stats);

    let errors = warp::path!("eth" / "tokens" / "api" / "runs" / String / "errors")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::errors);

    let stream = warp::path!("eth" / "tokens" / "api" / "runs" / String / "stream")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::stream);

    let stop = warp::path!("eth" / "tokens" / "api" / "runs" / String / "stop")
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
        .or(alpha_run_risks)
        .or(alpha_run_reports)
        .or(alpha_run_orders)
        .or(alpha_run_positions)
        .or(alpha_run_detail)
        .or(alpha_runs)
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
