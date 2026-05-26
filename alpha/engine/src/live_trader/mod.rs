use std::collections::HashMap;
use std::sync::atomic::Ordering;

use crate::{AlphaEngine, BlockCriticalRiskPolicy, EngineEvent};
use chrono::Utc;
use eth_alpha_store::PostgresTradingStore;
use eth_live_trading::StrategyGasRankPolicy;
use eth_strategies::shared_rules::live::observation_strategy_name;
use eyre::{eyre, Result, WrapErr};
use serde_json::{json, Value};
use tokio::time;
use tracing::{info, warn};

mod backtest;
#[path = "strategy/bankroll.rs"]
mod bankroll;
#[path = "config/cli.rs"]
mod cli;
#[path = "config/config_resolution.rs"]
mod config_resolution;
#[path = "config/constants.rs"]
mod constants;
#[path = "startup/entrypoints.rs"]
mod entrypoints;
mod event_processing;
mod execution_lifecycle;
#[path = "runtime/execution_stack.rs"]
mod execution_stack;
#[path = "strategy/gas_policy.rs"]
mod gas_policy;
#[path = "runtime/heartbeat.rs"]
mod heartbeat;
#[path = "state/live_state.rs"]
mod live_state;
#[path = "runtime/loop_control.rs"]
mod loop_control;
#[path = "operator/manual_close.rs"]
mod manual_close;
#[path = "risk/mined_pool_risks.rs"]
mod mined_pool_risks;
#[path = "runtime/poll_error.rs"]
mod poll_error;
#[path = "state/position_state.rs"]
mod position_state;
mod real_execution;
mod receipt_reconciliation;
#[path = "state/restored_state.rs"]
mod restored_state;
#[path = "risk/risk_annotation.rs"]
mod risk_annotation;
#[path = "startup/run_metadata.rs"]
mod run_metadata;
#[path = "startup/run_record.rs"]
mod run_record;
#[path = "startup/startup_log.rs"]
mod startup_log;
#[path = "strategy/definition.rs"]
mod strategy;
#[path = "strategy/setup.rs"]
mod strategy_setup;
#[path = "shared/support.rs"]
mod support;
#[path = "runtime/token_server.rs"]
mod token_server;

use bankroll::{
    entry_bankroll_summary_json, resolve_entry_bankroll_wei, single_strategy_value,
    validate_live_real_entry_bankrolls,
};
use cli::{Args, RealExecutionArgs};
use config_resolution::{resolve_cli_or_config_i64, resolve_cli_or_config_u64};
use constants::*;
use event_processing::{
    poll_live_inputs, process_pool_updates, process_position_monitor, reconcile_real_receipts,
    settle_chain_sim_executions, LivePollBatch, PoolUpdateProcessingInput, PositionMonitorInput,
};
use execution_stack::{build_execution_stack, ExecutionStackInput};
use gas_policy::load_live_real_gas_policy;
use heartbeat::{emit_tick_heartbeat, HeartbeatInput};
use loop_control::{
    mempool_signal_skip_reason_code_for_execution_mode, poll_error_retry_delay,
    preflight_chain_server_readiness, wait_for_next_loop_event,
};
use manual_close::{default_manual_close_limit, process_manual_close_requests};
use poll_error::handle_poll_error;
use real_execution::{
    parse_tail_entry_submission_route, preflight_kartal_real,
    validate_flashbots_tail_max_block_span, TailEntrySubmissionRoute,
};
use restored_state::restore_runtime_state;
use risk_annotation::{annotate_signal_risk_event, prime_projected_mempool_entry_pool};
use run_metadata::live_gas_policy_run_metadata_json;
use run_record::{record_alpha_trader_run_start, RunStartRecord};
use startup_log::{log_alpha_trader_start, StartupLogInput};
use strategy::build_strategy_specs;
use strategy_setup::install_live_strategies;
use support::*;
use token_server::TokenServerClient;

pub use entrypoints::{run_live_backtest, run_live_real};

async fn run(
    runner_name: &'static str,
    args: Args,
    execution_mode: TraderExecutionMode,
    real_args: Option<RealExecutionArgs>,
) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let shared_config = load_shared_config()?;
    let mempool_since_days = resolve_cli_or_config_i64(
        args.mempool_since_days,
        &shared_config,
        ALPHA_LIVE_MEMPOOL_SINCE_DAYS_CONFIG,
    )?;
    if mempool_since_days <= 0 {
        return Err(eyre!(
            "{} must be positive; got {}",
            ALPHA_LIVE_MEMPOOL_SINCE_DAYS_CONFIG,
            mempool_since_days
        ));
    }
    let signal_limit = resolve_cli_or_config_i64(
        args.signal_limit,
        &shared_config,
        ALPHA_LIVE_SIGNAL_LIMIT_CONFIG,
    )?;
    if signal_limit <= 0 {
        return Err(eyre!(
            "{} must be positive; got {}",
            ALPHA_LIVE_SIGNAL_LIMIT_CONFIG,
            signal_limit
        ));
    }
    if execution_mode.uses_kartal() != real_args.is_some() {
        return Err(eyre!(
            "{} internal configuration mismatch: execution mode {} and real args presence disagree",
            runner_name,
            execution_mode.label()
        ));
    }
    let tail_entry_submission_route = if execution_mode.uses_kartal() {
        let raw = shared_config
            .get(ALPHA_LIVE_TAIL_ENTRY_SUBMISSION_ROUTE_CONFIG)
            .map(String::as_str)
            .unwrap_or("flashbots_mev_share");
        Some(parse_tail_entry_submission_route(raw)?)
    } else {
        None
    };
    let flashbots_tail_max_block_span = if execution_mode.uses_kartal()
        && tail_entry_submission_route == Some(TailEntrySubmissionRoute::FlashbotsMevShare)
    {
        let span = resolve_cli_or_config_u64(
            None,
            &shared_config,
            ALPHA_LIVE_FLASHBOTS_TAIL_MAX_BLOCK_SPAN_CONFIG,
        )?;
        validate_flashbots_tail_max_block_span(span)?;
        Some(span)
    } else {
        None
    };
    let strategy_specs = build_strategy_specs(&args, execution_mode)?;
    let mut live_gas_policy = load_live_real_gas_policy(&shared_config)?;
    live_gas_policy.mempool_pre_mine_gas_rank_policy = StrategyGasRankPolicy::mempool_race_only();
    let live_real_gas_policy = if execution_mode.uses_kartal() {
        Some(live_gas_policy.clone())
    } else {
        None
    };
    let kartal_real_preflight = match real_args.as_ref() {
        None => None,
        Some(real_args) => Some(
            preflight_kartal_real(
                real_args,
                &args,
                &strategy_specs,
                tail_entry_submission_route
                    .expect("kartal-real execution requires tail-entry submission route"),
                flashbots_tail_max_block_span,
            )
            .await?,
        ),
    };
    let token_server_url = chain_server_url_from_config(&shared_config)?;
    let preflight_client = TokenServerClient::new(token_server_url.clone());
    preflight_chain_server_readiness(
        runner_name,
        &preflight_client,
        live_gas_policy.gas_rank_lookback_blocks as usize,
    )
    .await?;
    let reth_datadir = required_shared_config_value(&shared_config, RETH_DATADIR_CONFIG)?;
    let reth_http_rpc = required_shared_config_value(&shared_config, RETH_HTTP_RPC_CONFIG)?;
    let entry_bankrolls_wei = strategy_specs
        .iter()
        .map(resolve_entry_bankroll_wei)
        .collect::<Result<Vec<_>>>()?;
    if execution_mode.uses_kartal() && !args.disable_entry {
        validate_live_real_entry_bankrolls(
            runner_name,
            &strategy_specs,
            &entry_bankrolls_wei,
            LIVE_REAL_VALIDATION_MAX_ENTRY_BANKROLL_ETH,
        )?;
    }
    let entry_bankroll_summary = entry_bankroll_summary_json(&strategy_specs, &entry_bankrolls_wei);
    let single_entry_bankroll_wei = entry_bankrolls_wei.first().copied().flatten();
    let single_entry_bankroll_eth = if strategy_specs.len() == 1 {
        strategy_specs[0].entry_bankroll_eth.clone()
    } else {
        None
    };
    let single_buy_wei = single_strategy_value(&strategy_specs, |spec| spec.buy_wei.clone());
    let single_min_liquidity_eth =
        single_strategy_value(&strategy_specs, |spec| spec.min_liquidity_eth.clone());
    let single_min_liquidity_usd =
        single_strategy_value(&strategy_specs, |spec| spec.min_liquidity_usd.clone());
    let single_max_entry_pools = if strategy_specs.len() == 1 {
        strategy_specs[0].max_entry_pools
    } else {
        None
    };
    let observation_strategy_name = observation_strategy_name(&strategy_specs);
    let database_url = resolve_database_url(&shared_config)?;
    let run_id = args.run_id.clone().unwrap_or_else(default_run_id);
    let process_started_at = Utc::now();
    let process_started_at_text = process_started_at.to_rfc3339();
    let process_started_at_unix_secs = process_started_at.timestamp();
    let gas_policy_metadata = live_gas_policy_run_metadata_json(&live_gas_policy, execution_mode);
    if let Err(error) = init_alpha_trader_ops_events(&run_id, &shared_config) {
        warn!(error = %error, "failed to initialize alpha trader ops events");
    }

    let store = PostgresTradingStore::connect(&database_url, run_id.clone())
        .await
        .wrap_err("failed to initialize Postgres trading store")?;
    record_alpha_trader_run_start(
        &store,
        RunStartRecord {
            execution_mode,
            observation_strategy_name: &observation_strategy_name,
            strategy_specs: &strategy_specs,
            args: &args,
            real_args: real_args.as_ref(),
            process_started_at_text: &process_started_at_text,
            process_started_at_unix_secs,
            token_server_url: &token_server_url,
            reth_datadir: &reth_datadir,
            mempool_since_days,
            signal_limit,
            single_buy_wei: &single_buy_wei,
            single_max_entry_pools,
            single_entry_bankroll_eth: &single_entry_bankroll_eth,
            single_entry_bankroll_wei,
            entry_bankroll_summary: &entry_bankroll_summary,
            single_min_liquidity_eth: &single_min_liquidity_eth,
            single_min_liquidity_usd: &single_min_liquidity_usd,
            gas_policy_metadata,
            flashbots_tail_max_block_span,
        },
    )
    .await?;
    let stale_runs = store
        .mark_stale_runs(60)
        .await
        .wrap_err("failed to mark stale alpha trader runs")?;
    let restored_runtime = restore_runtime_state(&store, &strategy_specs).await?;
    let portfolio = restored_runtime.portfolio;
    let seen_pools_by_strategy = restored_runtime.seen_pools_by_strategy;
    let active_hold_counters_by_strategy = restored_runtime.active_hold_counters_by_strategy;
    let restored_entry_bankrolls_by_strategy = restored_runtime.entry_bankrolls_by_strategy;
    let restored_entry_bankroll_position_count = restored_runtime.entry_bankroll_position_count;
    let restored_entry_bankroll_accounted_pool_count =
        restored_runtime.entry_bankroll_accounted_pool_count;
    let restored_entry_bankroll_spent_wei = restored_runtime.entry_bankroll_spent_wei;
    let restored_entry_bankroll_recovered_wei = restored_runtime.entry_bankroll_recovered_wei;
    let restored_stale_submitted_positions = restored_runtime.stale_submitted_positions;
    let restored_position_count = restored_runtime.active_position_count;

    let execution_stack = build_execution_stack(ExecutionStackInput {
        execution_mode,
        real_args: real_args.as_ref(),
        token_server_url: &token_server_url,
        reth_datadir: &reth_datadir,
        reth_http_rpc: &reth_http_rpc,
        store: store.clone(),
        run_id: run_id.clone(),
        live_gas_policy: live_gas_policy.clone(),
        live_real_gas_policy: live_real_gas_policy.clone(),
        kartal_real_preflight,
        tail_entry_submission_route,
        flashbots_tail_max_block_span,
    })
    .await?;
    let adapter_current_block = execution_stack.adapter_current_block.clone();
    let pool_updates = execution_stack.pool_updates.clone();
    let manual_close_live_simulator = execution_stack.manual_close_live_simulator.clone();
    let state_status_adapter = execution_stack.state_status_adapter.clone();
    let manual_close_vault_address = execution_stack.manual_close_vault_address;
    let chain_sim_settlement = execution_stack.chain_sim_settlement;
    let receipt_reconciler = execution_stack.receipt_reconciler;
    let next_order_sequence = execution_stack.next_order_sequence;
    let adapter = execution_stack.adapter;

    let mut engine =
        AlphaEngine::new(BlockCriticalRiskPolicy, store.clone(), adapter).with_portfolio(portfolio);
    install_live_strategies(
        &mut engine,
        runner_name,
        &strategy_specs,
        &entry_bankrolls_wei,
        !args.disable_entry,
        &seen_pools_by_strategy,
        &active_hold_counters_by_strategy,
        &restored_entry_bankrolls_by_strategy,
    )?;

    let client = TokenServerClient::new(token_server_url.clone());
    let (mut seen_pool_blocks, mut seen_signal_ids, mut seen_mined_pool_risk_keys) =
        load_persisted_watermarks(&store, &observation_strategy_name).await?;
    let mut deferred_signal_wires = HashMap::new();
    let mut primed = false;
    let mut last_position_monitor_block: Option<u64> = None;
    let mut shutdown = ShutdownSignals::new()?;

    log_alpha_trader_start(StartupLogInput {
        token_server_url: &token_server_url,
        reth_datadir: &reth_datadir,
        run_id: &run_id,
        execution_mode,
        strategy_set: &args.strategy_set,
        strategy_count: strategy_specs.len(),
        observation_strategy_name: &observation_strategy_name,
        stale_runs,
        replay_current: args.replay_current,
        single_max_entry_pools,
        single_entry_bankroll_eth: &single_entry_bankroll_eth,
        single_entry_bankroll_wei,
        seen_pool_blocks: seen_pool_blocks.len(),
        seen_signal_ids: seen_signal_ids.len(),
        seen_mined_pool_risk_keys: seen_mined_pool_risk_keys.len(),
        restored_seen_pools: seen_pools_by_strategy.values().map(Vec::len).sum::<usize>(),
        restored_active_hold_counters: active_hold_counters_by_strategy
            .values()
            .map(Vec::len)
            .sum::<usize>(),
        restored_entry_bankroll_position_count,
        restored_entry_bankroll_accounted_pool_count,
        restored_entry_bankroll_spent_wei: &restored_entry_bankroll_spent_wei,
        restored_entry_bankroll_recovered_wei: &restored_entry_bankroll_recovered_wei,
        restored_stale_submitted_positions,
        restored_position_count,
        next_order_sequence,
        _entry_bankroll_summary: &entry_bankroll_summary,
    });

    loop {
        let first_poll = !primed;
        let poll_result =
            poll_live_inputs(&client, signal_limit, mempool_since_days, &pool_updates).await;
        let LivePollBatch {
            status,
            signals,
            pool_response_count,
            polled_pools,
            polled_pool_wires,
        } = match poll_result {
            Ok(result) => result,
            Err(error) => {
                let should_stop = handle_poll_error(
                    runner_name,
                    error,
                    &run_id,
                    &token_server_url,
                    &process_started_at_text,
                    process_started_at_unix_secs,
                    engine.portfolio().active_position_count(),
                    &store,
                    args.once,
                    &mut shutdown,
                    poll_error_retry_delay(execution_mode),
                )
                .await?;
                if should_stop {
                    break;
                }
                continue;
            }
        };
        let live_ready = status.progress.status == "live";
        let suppress_events = !args.replay_current && !live_ready;

        let mut market_events = 0usize;
        let mut risk_events = 0usize;
        let mut position_monitor_events = 0usize;
        let mut manual_close_requests = 0usize;
        let mut manual_close_failed = 0usize;
        let mut manual_close_reports = 0usize;
        let mut reports = 0usize;
        let chain_sim_summary = settle_chain_sim_executions(
            chain_sim_settlement.as_ref(),
            &status,
            suppress_events,
            &adapter_current_block,
            &mut engine,
        )
        .await?;
        reports += chain_sim_summary.total_reports;
        let chain_sim_settlement_loaded = chain_sim_summary.loaded;
        let chain_sim_settlement_reports = chain_sim_summary.reports;
        let chain_sim_settlement_pending = chain_sim_summary.pending;
        let chain_sim_settlement_waiting_state = chain_sim_summary.waiting_state;
        let chain_sim_settlement_missing_block = chain_sim_summary.missing_block;

        let receipt_summary = reconcile_real_receipts(
            receipt_reconciler.as_ref(),
            &store,
            &status,
            suppress_events,
            &mut engine,
        )
        .await?;
        reports += receipt_summary.total_reports;
        let receipt_reports = receipt_summary.reports;
        let receipt_unresolved = receipt_summary.unresolved;

        let defer_event_processing = !suppress_events && chain_sim_settlement_waiting_state > 0;
        if defer_event_processing {
            let mut deferred_signals = 0usize;
            for signal in signals.signals {
                if seen_signal_ids.contains(&signal.signal_id) {
                    continue;
                }
                deferred_signal_wires.insert(signal.signal_id.clone(), signal.clone());
                record_deferred_signal_observation(
                    &store,
                    &observation_strategy_name,
                    &signal,
                    "deferred",
                    first_poll,
                    suppress_events,
                    &status,
                    json!({
                        "reason_code": CHAIN_SIM_DEFER_EVENT_PROCESSING_REASON_CODE,
                        "chain_sim_settlement_waiting_state": chain_sim_settlement_waiting_state,
                        "chain_sim_settlement_loaded": chain_sim_settlement_loaded,
                        "live_current_block": status.progress.current_block,
                    }),
                )
                .await?;
                deferred_signals += 1;
            }
            warn!(
                chain_sim_settlement_waiting_state,
                chain_sim_settlement_loaded,
                deferred_signals,
                live_current_block = ?status.progress.current_block,
                reason_code = CHAIN_SIM_DEFER_EVENT_PROCESSING_REASON_CODE,
                "deferring live-backtest strategy events until exact chain-sim settlement state is available"
            );
        } else {
            let mut signal_wires = signals.signals;
            if !deferred_signal_wires.is_empty() {
                signal_wires.extend(deferred_signal_wires.drain().map(|(_, signal)| signal));
            }
            signal_wires.sort_by_key(|signal| signal.signal_id.parse::<u64>().unwrap_or(u64::MAX));
            for signal in signal_wires {
                let is_new = seen_signal_ids.insert(signal.signal_id.clone());
                if !is_new || suppress_events || (first_poll && !args.replay_current) {
                    if is_new {
                        record_signal_observation(
                            &store,
                            &observation_strategy_name,
                            &signal,
                            "primed",
                            0,
                            first_poll,
                            suppress_events,
                            &status,
                            Value::Null,
                        )
                        .await?;
                    }
                    continue;
                }
                if let Some(reason_code) = mempool_signal_skip_reason_code_for_execution_mode(
                    execution_mode,
                    &signal.signal_type,
                ) {
                    info!(
                        signal_id = %signal.signal_id,
                        signal_type = %signal.signal_type,
                        execution_mode = %execution_mode.label(),
                        reason_code = reason_code,
                        "skipping mempool signal for live trader execution mode"
                    );
                    record_signal_observation(
                        &store,
                        &observation_strategy_name,
                        &signal,
                        "ignored",
                        0,
                        first_poll,
                        suppress_events,
                        &status,
                        json!({
                            "reason_code": reason_code,
                            "execution_mode": execution_mode.label(),
                            "signal_type": signal.signal_type.as_str(),
                            "entry_path": MINED_POOL_RISK_SOURCE,
                        }),
                    )
                    .await?;
                    continue;
                }
                record_signal_observation(
                    &store,
                    &observation_strategy_name,
                    &signal,
                    "received",
                    0,
                    first_poll,
                    suppress_events,
                    &status,
                    json!({ "phase": "received", "reports": [] }),
                )
                .await?;
                let mut event = match signal.to_risk_event() {
                    Ok(Some(event)) => event,
                    Ok(None) => {
                        record_signal_observation(
                            &store,
                            &observation_strategy_name,
                            &signal,
                            "ignored",
                            0,
                            first_poll,
                            suppress_events,
                            &status,
                            json!({ "reason": "missing_token_address" }),
                        )
                        .await?;
                        continue;
                    }
                    Err(error) => {
                        record_signal_observation(
                            &store,
                            &observation_strategy_name,
                            &signal,
                            "invalid",
                            0,
                            first_poll,
                            suppress_events,
                            &status,
                            json!({ "error": error.to_string() }),
                        )
                        .await?;
                        warn!(error = %error, signal_id = %signal.signal_id, "skipping mempool signal");
                        continue;
                    }
                };
                let signal_block = event
                    .observed_block
                    .or(status.progress.current_block)
                    .unwrap_or_default();
                if signal_block > 0 {
                    if event.observed_block.is_none() {
                        event.observed_block = Some(signal_block);
                    }
                    adapter_current_block.store(signal_block, Ordering::Relaxed);
                }
                let pool_context = event
                    .pool_address
                    .as_ref()
                    .and_then(|pool_address| polled_pool_wires.get(pool_address));
                annotate_signal_risk_event(&mut event, &signal, pool_context);
                let projected_pool = prime_projected_mempool_entry_pool(
                    &event,
                    &pool_updates,
                    &adapter_current_block,
                );
                let event_reports = engine.handle_event(EngineEvent::Risk(event)).await?;
                let report_count = event_reports.len();
                let decision = if report_count > 0 {
                    "submitted"
                } else {
                    "hold"
                };
                record_signal_observation(
                    &store,
                    &observation_strategy_name,
                    &signal,
                    decision,
                    report_count,
                    first_poll,
                    suppress_events,
                    &status,
                    json!({
                        "reports": reports_payload(&event_reports),
                        "projected_pool_primed": projected_pool,
                    }),
                )
                .await?;
                reports += report_count;
                risk_events += 1;
                for report in event_reports {
                    info!(
                        order_id = %report.order_id.0,
                        status = ?report.status,
                        block_number = ?report.block_number,
                        gas_used = ?report.gas_used,
                        error = ?report.error,
                        "chain-sim execution report"
                    );
                }
            }
        }
        if !defer_event_processing {
            let pool_summary = process_pool_updates(
                PoolUpdateProcessingInput {
                    store: &store,
                    observation_strategy_name: &observation_strategy_name,
                    status: &status,
                    first_poll,
                    suppress_events,
                    replay_current: args.replay_current,
                    adapter_current_block: &adapter_current_block,
                },
                &mut engine,
                polled_pools,
                &mut seen_pool_blocks,
                &mut seen_mined_pool_risk_keys,
            )
            .await?;
            market_events += pool_summary.market_events;
            risk_events += pool_summary.risk_events;
            reports += pool_summary.reports;
        }

        if let Some(manual_close_live_simulator) = manual_close_live_simulator.as_ref() {
            if !defer_event_processing && !suppress_events && (!first_poll || args.replay_current) {
                match process_manual_close_requests(
                    &store,
                    &mut engine,
                    manual_close_live_simulator,
                    manual_close_vault_address,
                    status.progress.current_block,
                    default_manual_close_limit(),
                )
                .await
                {
                    Ok(summary) => {
                        manual_close_requests += summary.claimed;
                        manual_close_failed += summary.failed;
                        manual_close_reports += summary.reports;
                        reports += summary.reports;
                    }
                    Err(error) => {
                        warn!(error = %error, "manual close request processing failed");
                    }
                }
            }
        }

        if !defer_event_processing {
            let monitor_summary = process_position_monitor(
                PositionMonitorInput {
                    store: &store,
                    observation_strategy_name: &observation_strategy_name,
                    status: &status,
                    first_poll,
                    suppress_events,
                    replay_current: args.replay_current,
                    market_events,
                    adapter_current_block: &adapter_current_block,
                },
                &mut engine,
                &mut last_position_monitor_block,
            )
            .await?;
            position_monitor_events += monitor_summary.position_monitor_events;
            reports += monitor_summary.reports;
        }

        if first_poll && !args.replay_current {
            info!(
                pools = seen_pool_blocks.len(),
                signals = seen_signal_ids.len(),
                "primed alpha trader watermarks"
            );
        }
        primed = true;

        let chain_state_status = state_status_adapter.state_status().await;
        if let Err(error) = &chain_state_status {
            warn!(error = %error, "chain-sim state status unavailable");
        }
        let chain_state_payload = chain_state_status
            .as_ref()
            .ok()
            .map(|state| {
                json!({
                    "selected_block_number": state.selected_block_number,
                    "source": format!("{:?}", state.source),
                    "latest_reth_finished_block_number": state.latest_reth_finished_block_number,
                    "latest_historical_context_block_number": state.latest_historical_context_block_number,
                    "latest_live_block_number": state.latest_live_block_number,
                    "latest_tracked_state_block_number": state.latest_tracked_state_block_number,
                })
            });

        let heartbeat_metadata = emit_tick_heartbeat(HeartbeatInput {
            runner_name,
            execution_mode,
            chain_state_payload,
            process_started_at_text: &process_started_at_text,
            process_started_at_unix_secs,
            status: &status,
            live_ready,
            suppress_events,
            seen_pool_count: seen_pool_blocks.len(),
            pool_response_count,
            signal_count: signals.count,
            market_events,
            risk_events,
            position_monitor_events,
            manual_close_requests,
            manual_close_failed,
            manual_close_reports,
            chain_sim_settlement_loaded,
            chain_sim_settlement_reports,
            chain_sim_settlement_pending,
            chain_sim_settlement_waiting_state,
            chain_sim_settlement_missing_block,
            receipt_reports,
            receipt_unresolved,
            reports,
            strategy_count: strategy_specs.len(),
            observation_strategy_name: &observation_strategy_name,
            positions: engine.portfolio().active_position_count(),
            entry_enabled: !args.disable_entry,
            single_max_entry_pools,
            single_entry_bankroll_eth: &single_entry_bankroll_eth,
            single_entry_bankroll_wei,
            entry_bankroll_summary: &entry_bankroll_summary,
            run_id: &run_id,
            store: &store,
        })
        .await?;

        if args.once {
            store
                .mark_stopped("completed", heartbeat_metadata)
                .await
                .wrap_err("failed to mark alpha trader run completed")?;
            break;
        }
        tokio::select! {
            wait_result = wait_for_next_loop_event(&client, execution_mode, status.progress.current_block) => {
                if let Err(error) = wait_result {
                    warn!(
                        error = %error,
                        "alpha trader live-update wait failed; retrying"
                    );
                    time::sleep(poll_error_retry_delay(execution_mode)).await;
                }
            }
            _ = shutdown.recv() => {
                store
                    .mark_stopped(
                        "stopped",
                        json!({
                            "reason": "shutdown_signal",
                            "positions": engine.portfolio().active_position_count(),
                        }),
                    )
                    .await
                    .wrap_err("failed to mark alpha trader run stopped")?;
                break;
            }
        }
    }

    Ok(())
}
