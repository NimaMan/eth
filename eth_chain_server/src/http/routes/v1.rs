use warp::{Filter, Reply};

use super::{
    agent, alpha, backtest, health, live, mempool, ops, price, range, simulation, token_activity,
    token_analytics,
};
use crate::http::ServerState;
use crate::read_models::activity::TokenActivityBlocksQuery;
use crate::stores::alpha_trading::{
    ResultSetDetailQuery, ResultSetListQuery, ResultSetPerformanceQuery, ResultSetReportQuery,
    StrategyPerformanceQuery,
};
use crate::stores::mempool_signals::MempoolSignalQuery;

pub(super) fn routes(
    state: ServerState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let agent_manifest = warp::path!("api" / "v1" / "eth" / "agents" / "manifest")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(agent::manifest);

    let agent_status = warp::path!("api" / "v1" / "eth" / "agents" / "status")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(agent::status);

    let health = warp::path!("api" / "v1" / "eth" / "health")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(health::health);

    let live_status = warp::path!("api" / "v1" / "eth" / "live" / "status")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(live::status);

    let live_start = warp::path!("api" / "v1" / "eth" / "live" / "start")
        .and(warp::post())
        .and(warp::body::json())
        .and(super::with_state(state.clone()))
        .and_then(live::start);

    let live_stop = warp::path!("api" / "v1" / "eth" / "live" / "stop")
        .and(warp::post())
        .and(super::with_state(state.clone()))
        .and_then(live::stop);

    let live_tokens = warp::path!("api" / "v1" / "eth" / "live" / "tokens")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(live::tokens);

    let live_token_detail = warp::path!("api" / "v1" / "eth" / "live" / "tokens" / String)
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(live::token_detail);

    let live_surface = warp::path!("api" / "v1" / "eth" / "live" / "surface")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(live::surface);

    let live_pools = warp::path!("api" / "v1" / "eth" / "live" / "pools")
        .and(warp::get())
        .and(warp::query::<live::LivePoolsQuery>())
        .and(super::with_state(state.clone()))
        .and_then(live::pools);

    let live_active_pools = warp::path!("api" / "v1" / "eth" / "live" / "pools" / "active")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(live::active_pools);

    let live_scam_pools = warp::path!("api" / "v1" / "eth" / "live" / "pools" / "scam")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(live::scam_pools);

    let live_eligible_pools = warp::path!("api" / "v1" / "eth" / "live" / "pools" / "eligible")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(live::eligible_pools);

    let live_ineligible_pools = warp::path!("api" / "v1" / "eth" / "live" / "pools" / "ineligible")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(live::ineligible_pools);

    let live_updates = warp::path!("api" / "v1" / "eth" / "live" / "updates")
        .and(warp::get())
        .and(warp::query::<live::LiveUpdatesQuery>())
        .and(super::with_state(state.clone()))
        .and_then(live::updates);

    let live_retention = warp::path!("api" / "v1" / "eth" / "live" / "retention")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(live::retention);

    let live_processed_blocks = warp::path!("api" / "v1" / "eth" / "live" / "processed-blocks")
        .and(warp::get())
        .and(warp::query::<live::RecentProcessedBlocksQuery>())
        .and(super::with_state(state.clone()))
        .and_then(live::processed_blocks);

    let trading_live_status = warp::path!("api" / "v1" / "eth" / "trading" / "live" / "status")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(live::status);

    let trading_live_updates = warp::path!("api" / "v1" / "eth" / "trading" / "live" / "updates")
        .and(warp::get())
        .and(warp::query::<live::LiveUpdatesQuery>())
        .and(super::with_state(state.clone()))
        .and_then(live::updates);

    let trading_processed_blocks =
        warp::path!("api" / "v1" / "eth" / "trading" / "live" / "processed-blocks")
            .and(warp::get())
            .and(warp::query::<live::RecentProcessedBlocksQuery>())
            .and(super::with_state(state.clone()))
            .and_then(live::processed_blocks);

    let list_ranges = warp::path!("api" / "v1" / "eth" / "ranges")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(range::list_runs);

    let start_range = warp::path!("api" / "v1" / "eth" / "ranges")
        .and(warp::post())
        .and(warp::body::json())
        .and(super::with_state(state.clone()))
        .and_then(range::start_run);

    let active_range = warp::path!("api" / "v1" / "eth" / "ranges" / "active")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(range::active_run);

    let stop_active_range = warp::path!("api" / "v1" / "eth" / "ranges" / "active" / "stop")
        .and(warp::post())
        .and(super::with_state(state.clone()))
        .and_then(range::stop_active_run);

    let active_range_launch_stats =
        warp::path!("api" / "v1" / "eth" / "ranges" / "active" / "launch-stats")
            .and(warp::get())
            .and(super::with_state(state.clone()))
            .and_then(range::active_run_launch_stats);

    let cache_coverage = warp::path!("api" / "v1" / "eth" / "cache" / "coverage")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(range::processed_block_disk_cache_coverage);

    let price_spot = warp::path!("api" / "v1" / "eth" / "prices" / "spot")
        .and(warp::get())
        .and(warp::query::<price::SpotPriceQuery>())
        .and(super::with_state(state.clone()))
        .and_then(price::spot);

    let price_multi = warp::path!("api" / "v1" / "eth" / "prices" / "multi")
        .and(warp::get())
        .and(warp::query::<price::MultiPriceQuery>())
        .and(super::with_state(state.clone()))
        .and_then(price::multi);

    let price_stablecoins = warp::path!("api" / "v1" / "eth" / "prices" / "stablecoins")
        .and(warp::get())
        .and(warp::query::<price::StablecoinPriceQuery>())
        .and(super::with_state(state.clone()))
        .and_then(price::stablecoins);

    let price_swap_quote = warp::path!("api" / "v1" / "eth" / "prices" / "swap-quote")
        .and(warp::post())
        .and(warp::body::json())
        .and(super::with_state(state.clone()))
        .and_then(price::swap_quote);

    let range_progress = warp::path!("api" / "v1" / "eth" / "ranges" / String / "progress")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(range::progress);

    let range_tokens = warp::path!("api" / "v1" / "eth" / "ranges" / String / "tokens")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(range::tokens);

    let range_token_detail =
        warp::path!("api" / "v1" / "eth" / "ranges" / String / "tokens" / String)
            .and(warp::get())
            .and(super::with_state(state.clone()))
            .and_then(range::token_detail);

    let range_surface = warp::path!("api" / "v1" / "eth" / "ranges" / String / "surface")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(range::surface);

    let range_pools = warp::path!("api" / "v1" / "eth" / "ranges" / String / "pools")
        .and(warp::get())
        .and(warp::query::<range::RangePoolsQuery>())
        .and(super::with_state(state.clone()))
        .and_then(range::pools);

    let range_launch_stats = warp::path!("api" / "v1" / "eth" / "ranges" / String / "launch-stats")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(range::launch_stats);

    let range_risk_atlas_export =
        warp::path!("api" / "v1" / "eth" / "ranges" / String / "risk-atlas" / "export")
            .and(warp::post())
            .and(super::with_state(state.clone()))
            .and_then(range::export_risk_atlas);

    let range_errors = warp::path!("api" / "v1" / "eth" / "ranges" / String / "errors")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(range::errors);

    let range_stream = warp::path!("api" / "v1" / "eth" / "ranges" / String / "stream")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(range::stream);

    let range_stop = warp::path!("api" / "v1" / "eth" / "ranges" / String / "stop")
        .and(warp::post())
        .and(super::with_state(state.clone()))
        .and_then(range::stop_run);

    let ops_health = warp::path!("api" / "v1" / "eth" / "ops" / "health")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(ops::health);

    let ops_issues = warp::path!("api" / "v1" / "eth" / "ops" / "issues")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(ops::issues);

    let ops_bottlenecks = warp::path!("api" / "v1" / "eth" / "ops" / "bottlenecks")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(ops::bottlenecks);

    let mempool_signals = warp::path!("api" / "v1" / "eth" / "mempool" / "signals")
        .and(warp::get())
        .and(warp::query::<MempoolSignalQuery>())
        .and(super::with_state(state.clone()))
        .and_then(mempool::signals);

    let mempool_signals_by_type =
        warp::path!("api" / "v1" / "eth" / "mempool" / "signals" / String)
            .and(warp::get())
            .and(warp::query::<MempoolSignalQuery>())
            .and(super::with_state(state.clone()))
            .and_then(mempool::signals_by_type);

    let token_activity_blocks =
        warp::path!("api" / "v1" / "eth" / "tokens" / String / "activity-blocks")
            .and(warp::get())
            .and(warp::query::<TokenActivityBlocksQuery>())
            .and(super::with_state(state.clone()))
            .and_then(token_activity::activity_blocks);

    let risk_atlas = warp::path!("api" / "v1" / "eth" / "analytics" / "risk-atlas")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(token_analytics::risk_atlas);

    let risk_atlas_runs = warp::path!("api" / "v1" / "eth" / "analytics" / "risk-atlas" / "runs")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(token_analytics::risk_atlas_runs);

    let risk_atlas_run =
        warp::path!("api" / "v1" / "eth" / "analytics" / "risk-atlas" / "runs" / String)
            .and(warp::get())
            .and(super::with_state(state.clone()))
            .and_then(token_analytics::risk_atlas_run);

    let token_network_start = warp::path!("api" / "v1" / "eth" / "analytics" / "network")
        .and(warp::post())
        .and(warp::body::json())
        .and(super::with_state(state.clone()))
        .and_then(token_analytics::start);

    let token_network_list = warp::path!("api" / "v1" / "eth" / "analytics" / "network")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(token_analytics::list);

    let token_network_get = warp::path!("api" / "v1" / "eth" / "analytics" / "network" / String)
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(token_analytics::get);

    let token_network_cancel = warp::path!("api" / "v1" / "eth" / "analytics" / "network" / String)
        .and(warp::delete())
        .and(super::with_state(state.clone()))
        .and_then(token_analytics::cancel);

    let alpha_strategies = warp::path!("api" / "v1" / "eth" / "alpha" / "strategies")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(alpha::strategies);

    let alpha_strategy_detail = warp::path!("api" / "v1" / "eth" / "alpha" / "strategies" / String)
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(alpha::strategy_detail);

    let alpha_strategy_performance =
        warp::path!("api" / "v1" / "eth" / "alpha" / "strategies" / String / "performance")
            .and(warp::get())
            .and(warp::query::<StrategyPerformanceQuery>())
            .and(super::with_state(state.clone()))
            .and_then(alpha::strategy_performance);

    let alpha_strategy_reset =
        warp::path!("api" / "v1" / "eth" / "alpha" / "strategies" / String / "reset-state")
            .and(warp::post())
            .and(warp::body::json())
            .and(super::with_state(state.clone()))
            .and_then(alpha::strategy_reset);

    let alpha_gas_rank_estimate =
        warp::path!("api" / "v1" / "eth" / "alpha" / "gas-rank" / "estimate")
            .and(warp::post())
            .and(warp::body::json())
            .and(super::with_state(state.clone()))
            .and_then(alpha::gas_rank_estimate);

    let alpha_gas_rank_samples =
        warp::path!("api" / "v1" / "eth" / "alpha" / "gas-rank" / "samples")
            .and(warp::get())
            .and(warp::query::<
                crate::read_models::gas_rank::GasRankSamplesRequest,
            >())
            .and(super::with_state(state.clone()))
            .and_then(alpha::gas_rank_samples);

    let alpha_vault_simulation =
        warp::path!("api" / "v1" / "eth" / "alpha" / "simulations" / "vault")
            .and(warp::post())
            .and(warp::body::json())
            .and(super::with_state(state.clone()))
            .and_then(simulation::vault_uniswap_v2);

    let alpha_runs = warp::path!("api" / "v1" / "eth" / "alpha" / "runs")
        .and(warp::get())
        .and(warp::query::<backtest::RunListQuery>())
        .and(super::with_state(state.clone()))
        .and_then(backtest::list_runs);

    let alpha_result_sets = warp::path!("api" / "v1" / "eth" / "alpha" / "result-sets")
        .and(warp::get())
        .and(warp::query::<ResultSetListQuery>())
        .and(super::with_state(state.clone()))
        .and_then(backtest::result_sets);

    let alpha_run_detail = warp::path!("api" / "v1" / "eth" / "alpha" / "runs" / String)
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(backtest::run_detail);

    let alpha_result_set_detail =
        warp::path!("api" / "v1" / "eth" / "alpha" / "result-sets" / String)
            .and(warp::get())
            .and(warp::query::<ResultSetDetailQuery>())
            .and(super::with_state(state.clone()))
            .and_then(backtest::result_set_detail);

    let alpha_result_set_performance =
        warp::path!("api" / "v1" / "eth" / "alpha" / "result-sets" / String / "performance")
            .and(warp::get())
            .and(warp::query::<ResultSetPerformanceQuery>())
            .and(super::with_state(state.clone()))
            .and_then(backtest::result_set_performance);

    let alpha_result_set_strategies =
        warp::path!("api" / "v1" / "eth" / "alpha" / "result-sets" / String / "strategies")
            .and(warp::get())
            .and(super::with_state(state.clone()))
            .and_then(backtest::result_set_strategies);

    let alpha_result_set_validation =
        warp::path!("api" / "v1" / "eth" / "alpha" / "result-sets" / String / "validation")
            .and(warp::get())
            .and(warp::query::<ResultSetReportQuery>())
            .and(super::with_state(state.clone()))
            .and_then(backtest::result_set_validation);

    let alpha_result_set_assessment =
        warp::path!("api" / "v1" / "eth" / "alpha" / "result-sets" / String / "assessment")
            .and(warp::get())
            .and(warp::query::<ResultSetReportQuery>())
            .and(super::with_state(state.clone()))
            .and_then(backtest::result_set_assessment);

    let alpha_result_set_strategy_performance = warp::path!(
        "api"
            / "v1"
            / "eth"
            / "alpha"
            / "result-sets"
            / String
            / "strategies"
            / String
            / "performance"
    )
    .and(warp::get())
    .and(warp::query::<ResultSetPerformanceQuery>())
    .and(super::with_state(state.clone()))
    .and_then(backtest::result_set_strategy_performance);

    let alpha_result_set_strategy_validation = warp::path!(
        "api"
            / "v1"
            / "eth"
            / "alpha"
            / "result-sets"
            / String
            / "strategies"
            / String
            / "validation"
    )
    .and(warp::get())
    .and(warp::query::<ResultSetReportQuery>())
    .and(super::with_state(state.clone()))
    .and_then(backtest::result_set_strategy_validation);

    let alpha_result_set_strategy_assessment = warp::path!(
        "api"
            / "v1"
            / "eth"
            / "alpha"
            / "result-sets"
            / String
            / "strategies"
            / String
            / "assessment"
    )
    .and(warp::get())
    .and(warp::query::<ResultSetReportQuery>())
    .and(super::with_state(state.clone()))
    .and_then(backtest::result_set_strategy_assessment);

    let alpha_run_positions =
        warp::path!("api" / "v1" / "eth" / "alpha" / "runs" / String / "positions")
            .and(warp::get())
            .and(super::with_state(state.clone()))
            .and_then(backtest::run_positions);

    let alpha_run_orders = warp::path!("api" / "v1" / "eth" / "alpha" / "runs" / String / "orders")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(backtest::run_orders);

    let alpha_run_reports =
        warp::path!("api" / "v1" / "eth" / "alpha" / "runs" / String / "reports")
            .and(warp::get())
            .and(super::with_state(state.clone()))
            .and_then(backtest::run_execution_reports);

    let alpha_run_risks = warp::path!("api" / "v1" / "eth" / "alpha" / "runs" / String / "risks")
        .and(warp::get())
        .and(super::with_state(state.clone()))
        .and_then(backtest::run_risk_events);

    let alpha_run_decisions =
        warp::path!("api" / "v1" / "eth" / "alpha" / "runs" / String / "decisions")
            .and(warp::get())
            .and(super::with_state(state.clone()))
            .and_then(backtest::run_strategy_decisions);

    let alpha_run_decision_audit =
        warp::path!("api" / "v1" / "eth" / "alpha" / "runs" / String / "decision-audit")
            .and(warp::get())
            .and(super::with_state(state))
            .and_then(backtest::run_position_decision_audit);

    let agent_routes = agent_manifest.or(agent_status).boxed();

    let frontend_live_routes = live_status
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
        .boxed();

    let trading_routes = trading_live_status
        .or(trading_live_updates)
        .or(trading_processed_blocks)
        .boxed();

    let range_root_routes = list_ranges
        .or(start_range)
        .or(active_range)
        .or(stop_active_range)
        .or(active_range_launch_stats)
        .or(cache_coverage)
        .boxed();

    let price_routes = price_spot
        .or(price_multi)
        .or(price_stablecoins)
        .or(price_swap_quote)
        .boxed();

    let range_detail_routes = range_token_detail
        .or(range_tokens)
        .or(range_progress)
        .or(range_surface)
        .or(range_pools)
        .or(range_launch_stats)
        .or(range_risk_atlas_export)
        .or(range_errors)
        .or(range_stream)
        .or(range_stop)
        .boxed();

    let ops_routes = ops_health
        .or(ops_issues)
        .or(ops_bottlenecks)
        .or(mempool_signals_by_type)
        .or(mempool_signals)
        .or(token_activity_blocks)
        .or(risk_atlas_runs)
        .or(risk_atlas_run)
        .or(risk_atlas)
        .or(token_network_start)
        .or(token_network_list)
        .or(token_network_get)
        .or(token_network_cancel)
        .boxed();

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

    health
        .or(agent_routes)
        .or(frontend_live_routes)
        .or(trading_routes)
        .or(range_root_routes)
        .or(price_routes)
        .or(range_detail_routes)
        .or(ops_routes)
        .or(alpha_result_set_routes)
        .or(alpha_routes)
        .boxed()
}
