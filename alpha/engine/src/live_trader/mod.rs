use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::wire::{
    parse_address, LivePoolListResponse, LiveStatusResponse, MempoolSignalWire,
    MempoolSignalsResponse, PoolWire,
};
use crate::{
    AlphaEngine, BlockCriticalRiskPolicy, EngineEvent, EngineExecutionAdapter,
    LiveChainSimExecutionAdapter,
};
use alloy_primitives::U256;
use chrono::Utc;
use clap::Parser;
use eth_alpha_core::{
    amount::Amount,
    execution::ExecutionReport,
    ids::{StrategyName, TokenPoolId},
    market::{MarketEvent, PoolSnapshot},
    portfolio::PortfolioState,
    position::{Position, PositionState},
    store::TradingStore,
};
use eth_alpha_store::{PostgresTradingStore, StrategyObservationRecord};
use eth_ops_events::{
    emit_health, emit_issue, JsonlOpsEventSink, MultiOpsEventSink, PipelineHealth,
    PipelineHealthStatus, PipelineImpact, PipelineIssue, PipelineSeverity, TracingOpsEventSink,
};
use eth_strategies::shared_rules::live::{
    default_strategy_spec, observation_strategy_name, strategy_set_specs, LiveStrategySpec,
    LiveStrategySpecOptions, STRATEGY_RUNTIME,
};
use eth_strategies::{LiveSnipeAllConfig, LiveSnipeAllStrategy, SnipeAllConfig};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time;
use tracing::{info, warn};

mod cli;
mod position_state;
mod real_execution;
mod receipt_reconciliation;
mod strategy;
mod support;
mod token_server;

use cli::{parse_live_backtest_args, parse_live_real_args, Args, RealExecutionArgs};
use position_state::release_stale_submitted_position;
use real_execution::{build_kartal_real_adapter, preflight_kartal_real};
use receipt_reconciliation::{JsonRpcReceiptProvider, VaultReceiptReconciler};
use strategy::{build_strategy_specs, live_strategy_spec_config_json};
use support::*;
use token_server::TokenServerClient;

const POOL_UPDATE_SOURCE: &str = "pool_update";
const MEMPOOL_SIGNAL_SOURCE: &str = "mempool_signal";
const POSITION_MONITOR_SOURCE: &str = "position_monitor";
const ALPHA_DATABASE_URL_CONFIG: &str = "ALPHA_DATABASE_URL";
const ALPHA_TRADER_LOG_DIR_CONFIG: &str = "ALPHA_TRADER_LOG_DIR";
const CHAIN_SERVER_BIND_CONFIG: &str = "CHAIN_SERVER_BIND";
const RETH_DATADIR_CONFIG: &str = "RETH_DATADIR";
const DEFAULT_ALPHA_TRADER_LOG_DIR: &str =
    "/home/nima/code/crypto/blockchains/eth/logs/alpha_trader";
const DEFAULT_KARTAL_URL: &str = "http://127.0.0.1:5004";
const DEFAULT_KARTAL_TOKEN_ENV: &str = "ETH_TX_EXECUTOR_API_TOKEN";
const DEFAULT_LIVE_REAL_FROM: &str = "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27";
const DEFAULT_UNISWAP_V2_TRADING_VAULT: &str = "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597";

pub async fn run_live_backtest() -> Result<()> {
    run(
        "eth_alpha_live_backtest_trader",
        parse_live_backtest_args(),
        TraderExecutionMode::ChainSim,
        None,
    )
    .await
}

pub async fn run_live_real() -> Result<()> {
    let (args, real_args) = parse_live_real_args();
    run(
        "eth_alpha_live_trader",
        args,
        TraderExecutionMode::KartalReal,
        Some(real_args),
    )
    .await
}

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
    if execution_mode.uses_kartal() != real_args.is_some() {
        return Err(eyre!(
            "{} internal configuration mismatch: execution mode {} and real args presence disagree",
            runner_name,
            execution_mode.label()
        ));
    }
    if execution_mode.uses_kartal() && !args.disable_entry {
        return Err(eyre!(
            "{} requires --disable-entry until the vault buy path is wired into the real planner",
            runner_name
        ));
    }
    let mut kartal_real_preflight = match real_args.as_ref() {
        None => None,
        Some(real_args) => Some(preflight_kartal_real(real_args).await?),
    };
    let token_server_url = chain_server_url_from_config(&shared_config)?;
    let reth_datadir = required_shared_config_value(&shared_config, RETH_DATADIR_CONFIG)?;
    let buy_wei = parse_u256_decimal(&args.buy_wei)?;
    let min_liquidity_eth = Decimal::from_str(&args.min_liquidity_eth)
        .wrap_err("invalid --min-liquidity-eth decimal")?;
    let min_liquidity_usd = Decimal::from_str(&args.min_liquidity_usd)
        .wrap_err("invalid --min-liquidity-usd decimal")?;
    let strategy_specs = build_strategy_specs(&args)?;
    let observation_strategy_name = observation_strategy_name(&strategy_specs);
    let database_url = resolve_database_url(&shared_config)?;
    let run_id = args.run_id.clone().unwrap_or_else(default_run_id);
    let process_started_at = Utc::now();
    let process_started_at_text = process_started_at.to_rfc3339();
    let process_started_at_unix_secs = process_started_at.timestamp();
    if let Err(error) = init_alpha_trader_ops_events(&run_id, &shared_config) {
        warn!(error = %error, "failed to initialize alpha trader ops events");
    }

    let store = PostgresTradingStore::connect(&database_url, run_id.clone())
        .await
        .wrap_err("failed to initialize Postgres trading store")?;
    store
        .start_run(
            execution_mode.label(),
            json!({
                "strategy_name": &observation_strategy_name,
                "strategy_impl": if strategy_specs.len() == 1 { strategy_specs[0].strategy_impl.clone() } else { "multi-strategy-live-set".to_string() },
                "strategy_label": if strategy_specs.len() == 1 { strategy_specs[0].strategy_label.clone() } else { "Live Strategy Set".to_string() },
                "strategy_set": args.strategy_set.clone(),
                "strategy_suite": args.strategy_set.clone(),
                "strategy_count": strategy_specs.len(),
                "strategies": strategy_specs.iter().map(live_strategy_spec_config_json).collect::<Vec<_>>(),
                "strategy_runtime": STRATEGY_RUNTIME,
                "observation_strategy_name": &observation_strategy_name,
                "execution_model": execution_mode.execution_model(),
                "entry_enabled": !args.disable_entry,
                "kartal": if let Some(real_args) = real_args.as_ref() {
                    json!({
                        "url": &real_args.kartal_url,
                        "token_env": &real_args.kartal_token_env,
                        "from": &real_args.live_real_from,
                        "vault_address": &real_args.live_real_vault_address,
                        "broadcast_requirement": "dry_run"
                    })
                } else {
                    Value::Null
                },
                "process_started_at": &process_started_at_text,
                "process_started_at_unix_secs": process_started_at_unix_secs,
                "token_server_url": &token_server_url,
                "reth_datadir": &reth_datadir,
                "poll_interval_ms": args.poll_interval_ms,
                "mempool_since_days": args.mempool_since_days,
                "signal_limit": args.signal_limit,
                "buy_wei": &args.buy_wei,
                "min_liquidity_eth": &args.min_liquidity_eth,
                "min_liquidity_usd": &args.min_liquidity_usd,
                "replay_current": args.replay_current,
            }),
        )
        .await
        .wrap_err("failed to record alpha trader run")?;
    let stale_runs = store
        .mark_stale_runs(60)
        .await
        .wrap_err("failed to mark stale alpha trader runs")?;
    let mut portfolio = PortfolioState::default();
    let mut seen_pools_by_strategy = HashMap::new();
    let mut active_hold_counters_by_strategy = HashMap::new();
    let mut restored_stale_submitted_positions = 0usize;
    for spec in &strategy_specs {
        let seen_pools = store
            .load_seen_pools(&spec.strategy_name)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to restore seen alpha pools for {}",
                    spec.strategy_name
                )
            })?;
        seen_pools_by_strategy.insert(spec.strategy_name.clone(), seen_pools);

        let active_hold_counters = store
            .load_active_hold_counters(&spec.strategy_name)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to restore active hold counters for {}",
                    spec.strategy_name
                )
            })?;
        active_hold_counters_by_strategy.insert(spec.strategy_name.clone(), active_hold_counters);

        let restored_positions = store
            .load_active_positions(&spec.strategy_name)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to restore active alpha positions for {}",
                    spec.strategy_name
                )
            })?;
        for mut position in restored_positions {
            if release_stale_submitted_position(&mut position) {
                restored_stale_submitted_positions += 1;
                store.upsert_position(&position).await.wrap_err_with(|| {
                    format!(
                        "failed to persist stale submitted position recovery for {}",
                        position.id.0
                    )
                })?;
            }
            portfolio.positions.insert(position.id.clone(), position);
        }
    }
    let restored_position_count = portfolio.active_position_count();

    let live_simulator = tx_simulator::LiveTxSimulator::new(&reth_datadir)
        .wrap_err("failed to initialize live chain simulator")?;
    let tx_processor = Arc::new(tx_processor::tx_processor::TxProcessor::new());
    let next_order_sequence = store
        .max_order_sequence_for_prefix(&run_id)
        .await
        .wrap_err("failed to restore alpha trader order sequence")?;
    let chain_sim_adapter = LiveChainSimExecutionAdapter::with_prefix_and_next_order_sequence(
        live_simulator,
        tx_processor,
        run_id.clone(),
        next_order_sequence,
    )
    .wrap_err("failed to initialize chain-sim execution adapter")?;
    let adapter_current_block = chain_sim_adapter.current_block();
    let pool_updates = chain_sim_adapter.pools();
    let state_status_adapter = chain_sim_adapter.clone();
    let receipt_reconciler = match (
        execution_mode,
        real_args.as_ref(),
        kartal_real_preflight.as_ref(),
    ) {
        (TraderExecutionMode::KartalReal, Some(real_args), Some(preflight)) => {
            let vault = parse_address(&real_args.live_real_vault_address)?;
            Some(VaultReceiptReconciler::new(
                JsonRpcReceiptProvider::new(preflight.status.rpc_url.clone()),
                vault,
            ))
        }
        _ => None,
    };
    let adapter: Box<dyn EngineExecutionAdapter> = match execution_mode {
        TraderExecutionMode::ChainSim => Box::new(chain_sim_adapter),
        TraderExecutionMode::KartalReal => {
            let real_args = real_args
                .as_ref()
                .expect("kartal-real execution requires real args");
            build_kartal_real_adapter(
                real_args,
                kartal_real_preflight
                    .take()
                    .expect("kartal-real preflight must exist"),
                store.clone(),
                run_id.clone(),
                chain_sim_adapter,
                pool_updates.clone(),
                adapter_current_block.clone(),
            )
            .await?
        }
    };

    let mut engine =
        AlphaEngine::new(BlockCriticalRiskPolicy, store.clone(), adapter).with_portfolio(portfolio);
    for spec in &strategy_specs {
        let stop_loss_ratio = spec
            .stop_loss_ratio
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok());
        let take_profit_ratio = spec
            .take_profit_ratio
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok());
        let lp_approval_gate_min_pct = spec
            .lp_approval_gate_min_pct
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok());
        let min_sell_pool_denom_reserve = spec
            .min_sell_pool_denom_reserve
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or_else(|| SnipeAllConfig::default().min_sell_pool_denom_reserve);
        let config = LiveSnipeAllConfig::new(SnipeAllConfig {
            strategy_name: StrategyName(spec.strategy_name.clone()),
            buy_amount: Amount {
                raw: buy_wei,
                decimals: 18,
            },
            sell_fraction: eth_alpha_core::amount::DecimalAmount::from(1),
            min_denom_reserve: min_liquidity_eth,
            min_stable_denom_reserve: min_liquidity_usd,
            min_sell_pool_denom_reserve,
            entry_enabled: !args.disable_entry,
            stop_loss_ratio,
            take_profit_ratio,
            max_hold_blocks: spec.max_hold_blocks,
            exit_on_liquidity_removal: spec.exit_liquidity_removal,
            exit_on_tax: spec.exit_tax,
            exit_on_lp_approval: spec.exit_lp_approval,
            exit_on_critical_lp_approval_only: spec.exit_lp_approval_critical_only,
            exit_on_scam: spec.exit_scam,
            allowed_protocols: spec.allowed_protocols.clone(),
            block_entry_on_lp_approval: spec.block_entry_on_lp_approval,
            lp_approval_gate_min_pct,
            defer_buy_confirm_block_lp_approval_to_max_hold: spec
                .defer_buy_confirm_block_lp_approval_to_max_hold,
            ..SnipeAllConfig::default()
        });
        let seen_pools = seen_pools_by_strategy
            .get(&spec.strategy_name)
            .cloned()
            .unwrap_or_default();
        let active_hold_counters = active_hold_counters_by_strategy
            .get(&spec.strategy_name)
            .into_iter()
            .flat_map(|counters| counters.iter())
            .map(|counter| {
                (
                    counter.position_id.clone(),
                    counter.count,
                    counter.last_block,
                )
            })
            .collect::<Vec<_>>();
        engine.add_strategy(Box::new(LiveSnipeAllStrategy::with_restored_state(
            config,
            seen_pools,
            active_hold_counters,
        )));
    }

    let client = TokenServerClient::new(token_server_url.clone());
    let (mut seen_pool_blocks, mut seen_signal_ids) =
        load_persisted_watermarks(&store, &observation_strategy_name).await?;
    let mut primed = false;
    let mut last_position_monitor_block: Option<u64> = None;
    let mut shutdown = ShutdownSignals::new()?;

    info!(
        token_server_url = %token_server_url,
        reth_datadir = %reth_datadir,
        run_id = %run_id,
        mode = %execution_mode.label(),
        strategy_set = ?args.strategy_set,
        strategy_count = strategy_specs.len(),
        observation_strategy_name = %observation_strategy_name,
        stale_runs,
        replay_current = args.replay_current,
        restored_pool_watermarks = seen_pool_blocks.len(),
        restored_signal_watermarks = seen_signal_ids.len(),
        restored_seen_pools = seen_pools_by_strategy
            .values()
            .map(Vec::len)
            .sum::<usize>(),
        restored_active_hold_counters = active_hold_counters_by_strategy
            .values()
            .map(Vec::len)
            .sum::<usize>(),
        restored_stale_submitted_positions,
        restored_positions = restored_position_count,
        next_order_sequence,
        "starting alpha trader"
    );

    loop {
        let first_poll = !primed;
        let poll_result = async {
            let status = client.status().await?;
            let pools = client.pools().await?;
            let signals = client
                .mempool_signals(args.signal_limit, args.mempool_since_days)
                .await?;
            Ok::<_, eyre::Report>((status, pools, signals))
        }
        .await;
        let (status, pools, signals) = match poll_result {
            Ok(result) => result,
            Err(error) => {
                warn!(error = %error, "alpha trader poll failed");
                let mut issue = PipelineIssue::new(
                    runner_name,
                    "alpha_trader",
                    "token_server_poll",
                    PipelineSeverity::Warn,
                    PipelineImpact::ServiceDegraded,
                    "alpha_trader_poll_failed",
                    "Alpha trader token server poll failed",
                );
                issue.run_id = Some(run_id.clone());
                issue.retryable = true;
                issue.detail = Some(error.to_string());
                issue
                    .context
                    .insert("token_server_url".to_string(), json!(token_server_url));
                issue.context.insert(
                    "positions".to_string(),
                    json!(engine.portfolio().active_position_count()),
                );
                issue.refresh_ids();
                emit_issue(&issue);
                let mut health = PipelineHealth::new(
                    runner_name,
                    "alpha_trader",
                    "main_loop",
                    PipelineHealthStatus::Degraded,
                );
                health.run_id = Some(run_id.clone());
                health.metrics.insert(
                    "positions".to_string(),
                    json!(engine.portfolio().active_position_count()),
                );
                health
                    .metrics
                    .insert("poll_error".to_string(), json!(error.to_string()));
                emit_health(&health);
                let metadata = json!({
                    "token_server_url": &token_server_url,
                    "poll_error": error.to_string(),
                    "process_started_at": &process_started_at_text,
                    "process_started_at_unix_secs": process_started_at_unix_secs,
                    "trading_enabled": false,
                    "positions": engine.portfolio().active_position_count(),
                });
                store
                    .heartbeat(metadata.clone())
                    .await
                    .wrap_err("failed to write alpha trader error heartbeat")?;
                if args.once {
                    store
                        .mark_stopped("failed", metadata)
                        .await
                        .wrap_err("failed to mark alpha trader run failed")?;
                    break;
                }
                tokio::select! {
                    _ = time::sleep(Duration::from_millis(args.poll_interval_ms)) => {}
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
                continue;
            }
        };
        let live_ready = status.progress.status == "live";
        let suppress_events = !args.replay_current && !live_ready;

        let mut market_events = 0usize;
        let mut risk_events = 0usize;
        let mut position_monitor_events = 0usize;
        let mut reports = 0usize;
        let mut receipt_reports = 0usize;
        let mut receipt_unresolved = 0usize;

        let mut signal_wires = signals.signals;
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
                json!({ "reports": reports_payload(&event_reports) }),
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

        for pool_wire in pools.pools {
            let pool = match pool_wire.to_pool_snapshot() {
                Ok(pool) => pool,
                Err(error) => {
                    warn!(error = %error, "skipping pool snapshot");
                    continue;
                }
            };
            let previous_block = seen_pool_blocks.get(&pool.address).copied();
            pool_updates
                .lock()
                .expect("pool lock")
                .insert(pool.address.clone(), pool.clone());

            let changed = previous_block
                .map(|previous| pool.latest_block > previous)
                .unwrap_or(true);
            if !changed {
                continue;
            }
            seen_pool_blocks.insert(pool.address.clone(), pool.latest_block);

            if suppress_events || (first_poll && !args.replay_current) {
                record_pool_observation(
                    &store,
                    &observation_strategy_name,
                    &pool_wire,
                    &pool,
                    previous_block,
                    "primed",
                    0,
                    first_poll,
                    suppress_events,
                    &status,
                    Value::Null,
                )
                .await?;
                continue;
            }

            let event = MarketEvent::PoolUpdated {
                block_number: pool.latest_block,
                pool: pool.clone(),
            };
            adapter_current_block.store(pool.latest_block, Ordering::Relaxed);
            let event_reports = engine.handle_event(EngineEvent::Market(event)).await?;
            let report_count = event_reports.len();
            let decision = if report_count > 0 {
                "submitted"
            } else {
                "hold"
            };
            record_pool_observation(
                &store,
                &observation_strategy_name,
                &pool_wire,
                &pool,
                previous_block,
                decision,
                report_count,
                first_poll,
                suppress_events,
                &status,
                json!({ "reports": reports_payload(&event_reports) }),
            )
            .await?;
            reports += report_count;
            market_events += 1;
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

        if !suppress_events && (!first_poll || args.replay_current) {
            if let Some(block_number) = status.progress.current_block {
                let should_monitor = last_position_monitor_block
                    .map(|previous| block_number > previous)
                    .unwrap_or(true);
                if should_monitor {
                    let event = MarketEvent::BlockCompleted {
                        block_number,
                        updated_tokens: 0,
                        updated_pools: market_events,
                    };
                    adapter_current_block.store(block_number, Ordering::Relaxed);
                    let event_reports = engine.handle_event(EngineEvent::Market(event)).await?;
                    let report_count = event_reports.len();
                    record_position_monitor_observation(
                        &store,
                        &observation_strategy_name,
                        block_number,
                        "checked",
                        report_count,
                        first_poll,
                        suppress_events,
                        &status,
                        json!({ "reports": reports_payload(&event_reports) }),
                    )
                    .await?;
                    reports += report_count;
                    position_monitor_events += 1;
                    last_position_monitor_block = Some(block_number);
                    for report in event_reports {
                        info!(
                            order_id = %report.order_id.0,
                            status = ?report.status,
                            block_number = ?report.block_number,
                            gas_used = ?report.gas_used,
                            error = ?report.error,
                            "chain-sim position monitor execution report"
                        );
                    }
                }
            }
        }

        if let Some(reconciler) = &receipt_reconciler {
            match store.load_submitted_executions(50).await {
                Ok(submitted) if submitted.is_empty() => {}
                Ok(submitted) => match reconciler.reconcile(submitted).await {
                    Ok(batch) => {
                        receipt_unresolved = batch.unresolved.len();
                        for issue in batch.unresolved {
                            warn!(
                                order_id = %issue.order_id,
                                tx_hash = %issue.tx_hash,
                                reason = %issue.reason,
                                "real receipt reconciliation has no final vault evidence yet"
                            );
                        }
                        for report in batch.reports {
                            let event_reports =
                                engine.handle_event(EngineEvent::Execution(report)).await?;
                            receipt_reports += event_reports.len();
                            reports += event_reports.len();
                            for report in event_reports {
                                info!(
                                    order_id = %report.order_id.0,
                                    status = ?report.status,
                                    tx_hash = ?report.tx_hash,
                                    block_number = ?report.block_number,
                                    gas_used = ?report.gas_used,
                                    error = ?report.error,
                                    "real receipt reconciled execution report"
                                );
                            }
                        }
                    }
                    Err(error) => {
                        warn!(error = %error, "real receipt reconciliation failed");
                    }
                },
                Err(error) => {
                    warn!(error = %error, "failed to load submitted executions for receipt reconciliation");
                }
            }
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

        info!(
            live_status = %status.progress.status,
            live_current_block = ?status.progress.current_block,
            live_blocks_processed = status.progress.blocks_processed,
            live_warmup_total_blocks = status.progress.warmup_total_blocks,
            live_tracked_tokens = status.progress.tracked_tokens,
            live_tracked_pools = status.progress.tracked_pool_count(),
            live_tracked_v2_pools = status.progress.tracked_v2_pools,
            live_tracked_v3_pools = status.progress.tracked_v3_pools,
            live_tracked_v4_pools = status.progress.tracked_v4_pools,
            live_last_error = ?status.progress.last_error,
            trading_enabled = !suppress_events,
            pools_seen = seen_pool_blocks.len(),
            token_server_pool_count = pools.count,
            signal_count = signals.count,
            market_events,
            risk_events,
            position_monitor_events,
            receipt_reports,
            receipt_unresolved,
            reports,
            strategy_count = strategy_specs.len(),
            positions = engine.portfolio().active_position_count(),
            chain_sim_selected_block = chain_state_status.as_ref().ok().map(|state| state.selected_block_number),
            chain_sim_state_source = chain_state_status.as_ref().ok().map(|state| format!("{:?}", state.source)),
            "alpha trader tick"
        );
        let heartbeat_metadata = json!({
            "execution_model": execution_mode.execution_model(),
            "chain_sim_state": chain_state_payload,
            "process_started_at": &process_started_at_text,
            "process_started_at_unix_secs": process_started_at_unix_secs,
            "live_status": status.progress.status,
            "live_current_block": status.progress.current_block,
            "live_blocks_processed": status.progress.blocks_processed,
            "live_warmup_total_blocks": status.progress.warmup_total_blocks,
            "live_tracked_tokens": status.progress.tracked_tokens,
            "live_tracked_pools": status.progress.tracked_pool_count(),
            "live_tracked_v2_pools": status.progress.tracked_v2_pools,
            "live_tracked_v3_pools": status.progress.tracked_v3_pools,
            "live_tracked_v4_pools": status.progress.tracked_v4_pools,
            "live_last_error": status.progress.last_error,
            "trading_enabled": !suppress_events,
            "pools_seen": seen_pool_blocks.len(),
            "token_server_pool_count": pools.count,
            "signal_count": signals.count,
            "market_events": market_events,
            "risk_events": risk_events,
            "position_monitor_events": position_monitor_events,
            "receipt_reports": receipt_reports,
            "receipt_unresolved": receipt_unresolved,
            "reports": reports,
            "strategy_count": strategy_specs.len(),
            "observation_strategy_name": &observation_strategy_name,
            "positions": engine.portfolio().active_position_count(),
            "entry_enabled": !args.disable_entry,
            "kartal_enabled": execution_mode.uses_kartal(),
        });
        let mut health = PipelineHealth::new(
            runner_name,
            "alpha_trader",
            "main_loop",
            if live_ready {
                PipelineHealthStatus::Healthy
            } else {
                PipelineHealthStatus::Watch
            },
        );
        health.run_id = Some(run_id.clone());
        health.current_block = status.progress.current_block;
        health
            .metrics
            .insert("live_status".to_string(), json!(status.progress.status));
        health.metrics.insert(
            "live_blocks_processed".to_string(),
            json!(status.progress.blocks_processed),
        );
        health
            .metrics
            .insert("token_server_pool_count".to_string(), json!(pools.count));
        health
            .metrics
            .insert("signal_count".to_string(), json!(signals.count));
        health
            .metrics
            .insert("market_events".to_string(), json!(market_events));
        health
            .metrics
            .insert("risk_events".to_string(), json!(risk_events));
        health.metrics.insert(
            "position_monitor_events".to_string(),
            json!(position_monitor_events),
        );
        health
            .metrics
            .insert("receipt_reports".to_string(), json!(receipt_reports));
        health
            .metrics
            .insert("receipt_unresolved".to_string(), json!(receipt_unresolved));
        health.metrics.insert("reports".to_string(), json!(reports));
        health
            .metrics
            .insert("strategy_count".to_string(), json!(strategy_specs.len()));
        health.metrics.insert(
            "positions".to_string(),
            json!(engine.portfolio().active_position_count()),
        );
        health
            .metrics
            .insert("trading_enabled".to_string(), json!(!suppress_events));
        emit_health(&health);
        store
            .heartbeat(heartbeat_metadata.clone())
            .await
            .wrap_err("failed to write alpha trader heartbeat")?;

        if args.once {
            store
                .mark_stopped("completed", heartbeat_metadata)
                .await
                .wrap_err("failed to mark alpha trader run completed")?;
            break;
        }
        tokio::select! {
            _ = time::sleep(Duration::from_millis(args.poll_interval_ms)) => {}
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
