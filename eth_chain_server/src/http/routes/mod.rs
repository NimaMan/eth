mod agent;
mod alpha;
mod backtest;
mod health;
mod live;
mod mempool;
mod ops;
mod price;
mod range;
mod simulation;
mod token_activity;
mod token_analytics;
mod tx;
mod v1;

use std::convert::Infallible;

use warp::{Filter, Reply};

use crate::http::ServerState;
use crate::read_models::activity::TokenActivityBlocksQuery;
use crate::stores::alpha_trading::{
    ResultSetDetailQuery, ResultSetListQuery, ResultSetPerformanceQuery, ResultSetReportQuery,
    StrategyPerformanceQuery,
};
use crate::stores::mempool_signals::MempoolSignalQuery;

pub fn routes(
    state: ServerState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(["content-type"])
        .allow_methods(["GET", "POST", "DELETE", "OPTIONS"]);

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

    let live_status = warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "status")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::status);

    let live_start = warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "start")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(live::start);

    let live_stop = warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "stop")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(live::stop);

    let live_tokens = warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "tokens")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::tokens);

    let live_surface =
        warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "token-pool-surface")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(live::surface);

    let live_active_pools =
        warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "pools" / "active")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(live::active_pools);

    let live_scam_pools =
        warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "pools" / "scam")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(live::scam_pools);

    let live_eligible_pools =
        warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "pools" / "eligible")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(live::eligible_pools);

    let live_ineligible_pools =
        warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "pools" / "ineligible")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(live::ineligible_pools);

    let live_pools = warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "pools")
        .and(warp::get())
        .and(warp::query::<live::LivePoolsQuery>())
        .and(with_state(state.clone()))
        .and_then(live::pools);

    let live_updates =
        warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "block-applied-updates")
            .and(warp::get())
            .and(warp::query::<live::LiveUpdatesQuery>())
            .and(with_state(state.clone()))
            .and_then(live::updates);

    let live_token_detail =
        warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "tokens" / String)
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(live::token_detail);

    let live_retention = warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "retention")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(live::retention);

    let live_processed_blocks =
        warp::path!("eth" / "tokens" / "api" / "live-token-tracker" / "processed-blocks")
            .and(warp::get())
            .and(warp::query::<live::RecentProcessedBlocksQuery>())
            .and(with_state(state.clone()))
            .and_then(live::processed_blocks);

    let live_state_latest =
        warp::path!("eth" / "tokens" / "api" / "live-tx-simulator" / "latest-block-state")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(live::latest_state_frame);

    let live_tx_simulator_status =
        warp::path!("eth" / "tokens" / "api" / "live-tx-simulator" / "status")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(live::live_tx_simulator_status);

    let ops_health = warp::path!("eth" / "tokens" / "api" / "ops" / "health")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(ops::health);

    let ops_issues = warp::path!("eth" / "tokens" / "api" / "ops" / "issues")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(ops::issues);

    let ops_bottlenecks = warp::path!("eth" / "tokens" / "api" / "ops" / "bottlenecks")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(ops::bottlenecks);

    let mempool_signals =
        warp::path!("eth" / "tokens" / "api" / "mempool" / "pending-transaction-signals")
            .and(warp::get())
            .and(warp::query::<MempoolSignalQuery>())
            .and(with_state(state.clone()))
            .and_then(mempool::signals);

    let mempool_signals_by_type =
        warp::path!("eth" / "tokens" / "api" / "mempool" / "pending-transaction-signals" / String)
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

    let token_risk_atlas = warp::path!("eth" / "tokens" / "api" / "analytics" / "risk-atlas")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(token_analytics::risk_atlas);

    let token_risk_atlas_scammer_analytics =
        warp::path!("eth" / "tokens" / "api" / "analytics" / "risk-atlas" / "scammer-analytics")
            .and(warp::get())
            .and_then(token_analytics::scammer_analytics);

    let token_risk_atlas_scammer_analytics_alias =
        warp::path!("eth" / "tokens" / "api" / "analytics" / "risk-atlas" / "scammer_analytics")
            .and(warp::get())
            .and_then(token_analytics::scammer_analytics);

    let token_risk_atlas_runs =
        warp::path!("eth" / "tokens" / "api" / "analytics" / "risk-atlas" / "runs")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(token_analytics::risk_atlas_runs);

    let token_risk_atlas_run =
        warp::path!("eth" / "tokens" / "api" / "analytics" / "risk-atlas" / "runs" / String)
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(token_analytics::risk_atlas_run);

    let token_network_analysis_start =
        warp::path!("eth" / "tokens" / "api" / "analytics" / "network")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_state(state.clone()))
            .and_then(token_analytics::start);

    let token_network_analysis_list =
        warp::path!("eth" / "tokens" / "api" / "analytics" / "network")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(token_analytics::list);

    let token_network_analysis_get =
        warp::path!("eth" / "tokens" / "api" / "analytics" / "network" / String)
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(token_analytics::get);

    let token_network_analysis_cancel =
        warp::path!("eth" / "tokens" / "api" / "analytics" / "network" / String)
            .and(warp::delete())
            .and(with_state(state.clone()))
            .and_then(token_analytics::cancel);

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

    let alpha_gas_rank_estimate =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "gas-rank" / "estimate")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_state(state.clone()))
            .and_then(alpha::gas_rank_estimate);

    let alpha_gas_rank_samples =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "gas-rank" / "samples")
            .and(warp::get())
            .and(warp::query::<
                crate::read_models::gas_rank::GasRankSamplesRequest,
            >())
            .and(with_state(state.clone()))
            .and_then(alpha::gas_rank_samples);

    let alpha_vault_simulation =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "simulations" / "vault")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_state(state.clone()))
            .and_then(simulation::vault_uniswap_v2);

    let live_unsigned_simulation = warp::path!(
        "eth" / "tokens" / "api" / "live-tx-simulator" / "simulations" / "unsigned-transaction"
    )
    .and(warp::post())
    .and(warp::body::json())
    .and(with_state(state.clone()))
    .and_then(simulation::live_unsigned_tx);

    let live_unsigned_sequence_simulation = warp::path!(
        "eth"
            / "tokens"
            / "api"
            / "live-tx-simulator"
            / "simulations"
            / "unsigned-transaction-sequence"
    )
    .and(warp::post())
    .and(warp::body::json())
    .and(with_state(state.clone()))
    .and_then(simulation::live_unsigned_tx_sequence);

    let live_pool_buy_sell_simulation = warp::path!(
        "eth" / "tokens" / "api" / "live-tx-simulator" / "simulations" / "pool-buy-sell"
    )
    .and(warp::post())
    .and(warp::body::json())
    .and(with_state(state.clone()))
    .and_then(simulation::live_pool_buy_sell);

    let live_order_simulation =
        warp::path!("eth" / "tokens" / "api" / "live-tx-simulator" / "simulations" / "alpha-order")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_state(state.clone()))
            .and_then(simulation::live_order);

    let alpha_runs = warp::path!("eth" / "tokens" / "api" / "alpha" / "runs")
        .and(warp::get())
        .and(warp::query::<backtest::RunListQuery>())
        .and(with_state(state.clone()))
        .and_then(backtest::list_runs);

    let alpha_result_sets = warp::path!("eth" / "tokens" / "api" / "alpha" / "result-sets")
        .and(warp::get())
        .and(warp::query::<ResultSetListQuery>())
        .and(with_state(state.clone()))
        .and_then(backtest::result_sets);

    let alpha_run_detail = warp::path!("eth" / "tokens" / "api" / "alpha" / "runs" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(backtest::run_detail);

    let alpha_result_set_detail =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "result-sets" / String)
            .and(warp::get())
            .and(warp::query::<ResultSetDetailQuery>())
            .and(with_state(state.clone()))
            .and_then(backtest::result_set_detail);

    let alpha_result_set_performance =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "result-sets" / String / "performance")
            .and(warp::get())
            .and(warp::query::<ResultSetPerformanceQuery>())
            .and(with_state(state.clone()))
            .and_then(backtest::result_set_performance);

    let alpha_result_set_strategies =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "result-sets" / String / "strategies")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(backtest::result_set_strategies);

    let alpha_result_set_validation =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "result-sets" / String / "validation")
            .and(warp::get())
            .and(warp::query::<ResultSetReportQuery>())
            .and(with_state(state.clone()))
            .and_then(backtest::result_set_validation);

    let alpha_result_set_assessment =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "result-sets" / String / "assessment")
            .and(warp::get())
            .and(warp::query::<ResultSetReportQuery>())
            .and(with_state(state.clone()))
            .and_then(backtest::result_set_assessment);

    let alpha_result_set_strategy_performance = warp::path!(
        "eth"
            / "tokens"
            / "api"
            / "alpha"
            / "result-sets"
            / String
            / "strategies"
            / String
            / "performance"
    )
    .and(warp::get())
    .and(warp::query::<ResultSetPerformanceQuery>())
    .and(with_state(state.clone()))
    .and_then(backtest::result_set_strategy_performance);

    let alpha_result_set_strategy_validation = warp::path!(
        "eth"
            / "tokens"
            / "api"
            / "alpha"
            / "result-sets"
            / String
            / "strategies"
            / String
            / "validation"
    )
    .and(warp::get())
    .and(warp::query::<ResultSetReportQuery>())
    .and(with_state(state.clone()))
    .and_then(backtest::result_set_strategy_validation);

    let alpha_result_set_strategy_assessment = warp::path!(
        "eth"
            / "tokens"
            / "api"
            / "alpha"
            / "result-sets"
            / String
            / "strategies"
            / String
            / "assessment"
    )
    .and(warp::get())
    .and(warp::query::<ResultSetReportQuery>())
    .and(with_state(state.clone()))
    .and_then(backtest::result_set_strategy_assessment);

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

    let alpha_run_decisions =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "runs" / String / "decisions")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(backtest::run_strategy_decisions);

    let alpha_run_decision_audit =
        warp::path!("eth" / "tokens" / "api" / "alpha" / "runs" / String / "decision-audit")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(backtest::run_position_decision_audit);

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

    let surface = warp::path!("eth" / "tokens" / "api" / "runs" / String / "surface")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::surface);

    let pools = warp::path!("eth" / "tokens" / "api" / "runs" / String / "pools")
        .and(warp::get())
        .and(warp::query::<range::RangePoolsQuery>())
        .and(with_state(state.clone()))
        .and_then(range::pools);

    let launch_stats =
        warp::path!("eth" / "tokens" / "api" / "runs" / String / "strategy" / "launch-stats")
            .and(warp::get())
            .and(with_state(state.clone()))
            .and_then(range::launch_stats);

    let risk_atlas_export =
        warp::path!("eth" / "tokens" / "api" / "runs" / String / "risk-atlas" / "export")
            .and(warp::post())
            .and(with_state(state.clone()))
            .and_then(range::export_risk_atlas);

    let errors = warp::path!("eth" / "tokens" / "api" / "runs" / String / "errors")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::errors);

    let stream = warp::path!("eth" / "tokens" / "api" / "runs" / String / "progress-stream")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(range::stream);

    let stop = warp::path!("eth" / "tokens" / "api" / "runs" / String / "stop")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(range::stop_run);

    let alpha_result_set_routes = alpha_result_set_strategy_assessment
        .or(alpha_result_set_strategy_validation)
        .or(alpha_result_set_strategy_performance)
        .or(alpha_result_set_assessment)
        .or(alpha_result_set_validation)
        .or(alpha_result_set_strategies)
        .or(alpha_result_set_performance)
        .or(alpha_result_set_detail)
        .or(alpha_result_sets)
        .boxed();

    let token_run_routes = list_runs
        .or(start_run)
        .or(active_run)
        .or(stop_active_run)
        .or(active_run_launch_stats)
        .or(processed_block_disk_cache_coverage)
        .boxed();

    let live_routes = live_status
        .or(live_start)
        .or(live_stop)
        .or(live_token_detail)
        .or(live_tokens)
        .or(live_surface)
        .or(live_active_pools)
        .or(live_scam_pools)
        .or(live_eligible_pools)
        .or(live_ineligible_pools)
        .or(live_pools)
        .or(live_updates)
        .or(live_retention)
        .or(live_processed_blocks)
        .or(live_tx_simulator_status)
        .or(live_unsigned_simulation)
        .or(live_unsigned_sequence_simulation)
        .or(live_pool_buy_sell_simulation)
        .or(live_order_simulation)
        .or(live_state_latest)
        .boxed();

    let ops_routes = ops_health
        .or(ops_issues)
        .or(ops_bottlenecks)
        .or(mempool_signals_by_type)
        .or(mempool_signals)
        .or(token_activity_blocks)
        .or(token_risk_atlas_scammer_analytics)
        .or(token_risk_atlas_scammer_analytics_alias)
        .or(token_risk_atlas_runs)
        .or(token_risk_atlas_run)
        .or(token_risk_atlas)
        .or(token_network_analysis_start)
        .or(token_network_analysis_list)
        .or(token_network_analysis_get)
        .or(token_network_analysis_cancel)
        .boxed();

    let alpha_routes = alpha_strategy_performance
        .or(alpha_strategy_reset)
        .or(alpha_vault_simulation)
        .or(alpha_gas_rank_samples)
        .or(alpha_gas_rank_estimate)
        .or(alpha_strategy_detail)
        .or(alpha_strategies)
        .or(alpha_run_decision_audit)
        .or(alpha_run_decisions)
        .or(alpha_run_risks)
        .or(alpha_run_reports)
        .or(alpha_run_orders)
        .or(alpha_run_positions)
        .or(alpha_run_detail)
        .or(alpha_runs)
        .boxed();

    let range_routes = token_detail
        .or(tokens)
        .or(progress)
        .or(surface)
        .or(pools)
        .or(launch_stats)
        .or(risk_atlas_export)
        .or(errors)
        .or(stream)
        .or(stop)
        .boxed();

    let legacy_routes = health
        .or(alpha_result_set_routes)
        .or(token_run_routes)
        .or(live_routes)
        .or(ops_routes)
        .or(alpha_routes)
        .or(range_routes)
        .boxed();

    legacy_routes.or(v1::routes(state)).boxed()
}

fn with_state(
    state: ServerState,
) -> impl Filter<Extract = (ServerState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}
