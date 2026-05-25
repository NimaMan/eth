use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::wire::{
    parse_address, LivePoolListResponse, LiveStatusResponse, MempoolSignalsResponse,
};
use crate::{
    AlphaEngine, BlockCriticalRiskPolicy, EngineEvent, EngineExecutionAdapter,
    LiveChainSimExecutionAdapter,
};
use chrono::Utc;
use clap::Parser;
use eth_alpha_core::market::MarketEvent;
use eth_alpha_store::PostgresTradingStore;
use eth_live_trading::StrategyGasRankPolicy;
use eth_ops_events::{emit_health, PipelineHealth, PipelineHealthStatus};
use eth_strategies::shared_rules::live::{observation_strategy_name, STRATEGY_RUNTIME};
use eyre::{eyre, Result, WrapErr};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time;
use tracing::{info, warn};

mod backtest;
mod bankroll;
mod cli;
mod config_resolution;
mod entrypoints;
mod gas_policy;
mod live_state;
mod manual_close;
mod mined_pool_risks;
mod poll_error;
mod position_state;
mod real_execution;
mod receipt_reconciliation;
mod restored_state;
mod risk_annotation;
mod run_metadata;
mod strategy;
mod strategy_setup;
mod support;
mod token_server;

use backtest::ChainSimGasPolicyBacktestAdapter;
use bankroll::{entry_bankroll_summary_json, resolve_entry_bankroll_wei, single_strategy_value};
use cli::{Args, RealExecutionArgs};
use config_resolution::{resolve_cli_or_config_i64, resolve_cli_or_config_u64};
use gas_policy::load_live_real_gas_policy;
use live_state::spawn_live_state_publisher;
use manual_close::{default_manual_close_limit, process_manual_close_requests};
use mined_pool_risks::mined_pool_risks_from_update;
use poll_error::handle_poll_error;
use real_execution::{
    build_kartal_real_adapter, preflight_kartal_real, validate_flashbots_tail_max_block_span,
};
use receipt_reconciliation::{JsonRpcReceiptProvider, VaultReceiptReconciler};
use restored_state::restore_runtime_state;
use risk_annotation::{annotate_signal_risk_event, prime_projected_mempool_entry_pool};
use run_metadata::live_gas_policy_run_metadata_json;
use strategy::{build_strategy_specs, live_strategy_spec_config_json};
use strategy_setup::{build_live_strategy, LiveStrategyRestore};
use support::*;
use token_server::TokenServerClient;

pub use entrypoints::{run_live_backtest, run_live_real};

const POOL_UPDATE_SOURCE: &str = "pool_update";
const MEMPOOL_SIGNAL_SOURCE: &str = "mempool_signal";
const MINED_POOL_RISK_SOURCE: &str = "mined_pool_update";
const POSITION_MONITOR_SOURCE: &str = "position_monitor";
const ALPHA_DATABASE_CONFIG_KEY: &str = "databases.alpha.url";
const ALPHA_TRADER_LOG_DIR_CONFIG: &str = "ALPHA_TRADER_LOG_DIR";
const ALPHA_LIVE_TRADER_POLL_INTERVAL_MS_CONFIG: &str = "ALPHA_LIVE_TRADER_POLL_INTERVAL_MS";
const ALPHA_LIVE_MEMPOOL_SINCE_DAYS_CONFIG: &str = "ALPHA_LIVE_MEMPOOL_SINCE_DAYS";
const ALPHA_LIVE_SIGNAL_LIMIT_CONFIG: &str = "ALPHA_LIVE_SIGNAL_LIMIT";
const ALPHA_LIVE_FLASHBOTS_TAIL_MAX_BLOCK_SPAN_CONFIG: &str =
    "ALPHA_LIVE_FLASHBOTS_TAIL_MAX_BLOCK_SPAN";
const CHAIN_SERVER_BIND_CONFIG: &str = "CHAIN_SERVER_BIND";
const RETH_DATADIR_CONFIG: &str = "RETH_DATADIR";
const RETH_HTTP_RPC_CONFIG: &str = "RETH_HTTP_RPC";
const DEFAULT_ALPHA_TRADER_LOG_DIR: &str =
    "/home/nima/code/crypto/blockchains/eth/logs/alpha_trader";
const DEFAULT_KARTAL_URL: &str = "http://127.0.0.1:5004";
const DEFAULT_KARTAL_TOKEN_ENV: &str = "ETH_TX_EXECUTOR_API_TOKEN";
const DEFAULT_LIVE_REAL_FROM: &str = "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27";
const DEFAULT_UNISWAP_V2_TRADING_VAULT: &str = "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597";
const LIVE_REAL_VALIDATION_MAX_ENTRY_BANKROLL_ETH: &str = "0.555";
const MAX_LIVE_TRADER_POLL_INTERVAL_MS: u64 = 1_000;
const CHAIN_SIM_SKIP_MEMPOOL_TRADING_ENABLED_REASON: &str =
    "chain_sim.live_backtest.skip_mempool_trading_enabled";

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
    let poll_interval_ms = resolve_cli_or_config_u64(
        args.poll_interval_ms,
        &shared_config,
        ALPHA_LIVE_TRADER_POLL_INTERVAL_MS_CONFIG,
    )?;
    if poll_interval_ms == 0 || poll_interval_ms > MAX_LIVE_TRADER_POLL_INTERVAL_MS {
        return Err(eyre!(
            "{} must be in the range 1..={}ms for live mempool signal handling; got {}",
            ALPHA_LIVE_TRADER_POLL_INTERVAL_MS_CONFIG,
            MAX_LIVE_TRADER_POLL_INTERVAL_MS,
            poll_interval_ms
        ));
    }
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
    let flashbots_tail_max_block_span = if execution_mode.uses_kartal() {
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
    let mut kartal_real_preflight = match real_args.as_ref() {
        None => None,
        Some(real_args) => {
            let flashbots_tail_max_block_span = flashbots_tail_max_block_span
                .expect("kartal-real execution requires Flashbots tail span config");
            Some(
                preflight_kartal_real(
                    real_args,
                    &args,
                    &strategy_specs,
                    flashbots_tail_max_block_span,
                )
                .await?,
            )
        }
    };
    let token_server_url = chain_server_url_from_config(&shared_config)?;
    let reth_datadir = required_shared_config_value(&shared_config, RETH_DATADIR_CONFIG)?;
    let reth_http_rpc = required_shared_config_value(&shared_config, RETH_HTTP_RPC_CONFIG)?;
    let entry_bankrolls_wei = strategy_specs
        .iter()
        .map(resolve_entry_bankroll_wei)
        .collect::<Result<Vec<_>>>()?;
    if execution_mode.uses_kartal() && !args.disable_entry {
        let validation_limit = parse_eth_decimal_to_wei(
            LIVE_REAL_VALIDATION_MAX_ENTRY_BANKROLL_ETH,
            "live-real validation entry bankroll",
        )?;
        for (spec, bankroll) in strategy_specs.iter().zip(entry_bankrolls_wei.iter()) {
            let bankroll = bankroll.ok_or_else(|| {
                eyre!(
                    "{} requires an entry bankroll <= {} for strategy {} while live-real entries are in validation mode",
                    runner_name,
                    LIVE_REAL_VALIDATION_MAX_ENTRY_BANKROLL_ETH,
                    spec.strategy_name
                )
            })?;
            if bankroll.is_zero() || bankroll > validation_limit {
                return Err(eyre!(
                    "{} requires entry bankroll in the range (0, {}] for strategy {}; got spec {:?}",
                    runner_name,
                    LIVE_REAL_VALIDATION_MAX_ENTRY_BANKROLL_ETH,
                    spec.strategy_name,
                    &spec.entry_bankroll_eth
                ));
            }
        }
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
    store
        .start_run(
            execution_mode.label(),
            json!({
                "strategy_name": &observation_strategy_name,
                "strategy_impl": if strategy_specs.len() == 1 { strategy_specs[0].strategy_impl.clone() } else { "multi-strategy-set".to_string() },
                "strategy_label": if strategy_specs.len() == 1 { strategy_specs[0].strategy_label.clone() } else { "Strategy Set".to_string() },
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
                        "broadcast_requirement": if real_args.allow_public_mempool_live_validation {
                            "dry_run_or_explicit_hold16_deploy_public_mempool"
                        } else {
                            "dry_run"
                        },
                        "allow_public_mempool_live_validation": real_args.allow_public_mempool_live_validation,
                        "flashbots_submission_owner": "kartal",
                        "flashbots_tail_max_block_span": flashbots_tail_max_block_span
                    })
                } else {
                    Value::Null
                },
                "process_started_at": &process_started_at_text,
                "process_started_at_unix_secs": process_started_at_unix_secs,
                "token_server_url": &token_server_url,
                "reth_datadir": &reth_datadir,
                "poll_interval_ms": poll_interval_ms,
                "mempool_since_days": mempool_since_days,
                "signal_limit": signal_limit,
                "buy_wei": &single_buy_wei,
                "max_entry_pools": single_max_entry_pools,
                "entry_bankroll_eth": &single_entry_bankroll_eth,
                "entry_bankroll_wei": single_entry_bankroll_wei.map(|value| value.to_string()),
                "entry_bankrolls": &entry_bankroll_summary,
                "min_liquidity_eth": &single_min_liquidity_eth,
                "min_liquidity_usd": &single_min_liquidity_usd,
                "replay_current": args.replay_current,
                "gas_policy": gas_policy_metadata,
            }),
        )
        .await
        .wrap_err("failed to record alpha trader run")?;
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

    let tx_simulator = Arc::new(
        tx_simulator::TxSimulator::new(&reth_datadir)
            .wrap_err("failed to initialize tx simulator")?,
    );
    let live_state_provider = tx_simulator::InMemoryLiveBlockStateProvider::new();
    let live_simulator =
        tx_simulator::LiveTxSimulator::new(tx_simulator.clone(), live_state_provider.clone());
    spawn_live_state_publisher(
        TokenServerClient::new(token_server_url.clone()),
        live_state_provider,
        tx_simulator.clone(),
    );
    let tx_processor = Arc::new(tx_processor::tx_processor::TxProcessor::new());
    let next_order_sequence = store
        .max_order_sequence_for_prefix(&run_id)
        .await
        .wrap_err("failed to restore alpha trader order sequence")?;
    let chain_sim_adapter = LiveChainSimExecutionAdapter::with_prefix_and_next_order_sequence(
        live_simulator.clone(),
        tx_processor,
        run_id.clone(),
        next_order_sequence,
    )
    .wrap_err("failed to initialize chain-sim execution adapter")?;
    let adapter_current_block = chain_sim_adapter.current_block();
    let pool_updates = chain_sim_adapter.pools();
    let exact_pre_submit_live_simulator = Some(live_simulator.clone());
    let manual_close_live_simulator = exact_pre_submit_live_simulator.clone();
    let state_status_adapter = chain_sim_adapter.clone();
    let manual_close_vault_address = match (execution_mode, real_args.as_ref()) {
        (TraderExecutionMode::KartalReal, Some(real_args)) => {
            Some(parse_address(&real_args.live_real_vault_address)?)
        }
        _ => None,
    };
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
        TraderExecutionMode::ChainSim => Box::new(ChainSimGasPolicyBacktestAdapter::new(
            chain_sim_adapter,
            token_server_url.clone(),
            reth_http_rpc.clone(),
            live_gas_policy.clone(),
        )),
        TraderExecutionMode::KartalReal => {
            let real_args = real_args
                .as_ref()
                .expect("kartal-real execution requires real args");
            build_kartal_real_adapter(
                real_args,
                kartal_real_preflight
                    .take()
                    .expect("kartal-real preflight must exist"),
                token_server_url.clone(),
                store.clone(),
                run_id.clone(),
                chain_sim_adapter,
                exact_pre_submit_live_simulator,
                pool_updates.clone(),
                adapter_current_block.clone(),
                flashbots_tail_max_block_span
                    .expect("kartal-real execution requires Flashbots tail span config"),
                live_real_gas_policy
                    .clone()
                    .expect("kartal-real gas policy must exist"),
            )
            .await?
        }
    };

    let mut engine =
        AlphaEngine::new(BlockCriticalRiskPolicy, store.clone(), adapter).with_portfolio(portfolio);
    for (spec, entry_bankroll_wei) in strategy_specs
        .iter()
        .zip(entry_bankrolls_wei.iter().copied())
    {
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
        let restored_entry_bankroll = restored_entry_bankrolls_by_strategy
            .get(&spec.strategy_name)
            .cloned()
            .unwrap_or_default();
        let strategy = build_live_strategy(
            runner_name,
            spec,
            entry_bankroll_wei,
            !args.disable_entry,
            LiveStrategyRestore {
                seen_pools,
                active_hold_counters,
                entry_bankroll: restored_entry_bankroll,
            },
        )?;
        engine.add_strategy(strategy);
    }

    let client = TokenServerClient::new(token_server_url.clone());
    let (mut seen_pool_blocks, mut seen_signal_ids, mut seen_mined_pool_risk_keys) =
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
        max_entry_pools = ?single_max_entry_pools,
        entry_bankroll_eth = ?single_entry_bankroll_eth,
        entry_bankroll_wei = ?single_entry_bankroll_wei.map(|value| value.to_string()),
        restored_pool_watermarks = seen_pool_blocks.len(),
        restored_signal_watermarks = seen_signal_ids.len(),
        restored_mined_pool_risk_watermarks = seen_mined_pool_risk_keys.len(),
        restored_seen_pools = seen_pools_by_strategy
            .values()
            .map(Vec::len)
            .sum::<usize>(),
        restored_active_hold_counters = active_hold_counters_by_strategy
            .values()
            .map(Vec::len)
            .sum::<usize>(),
        restored_entry_bankroll_positions = restored_entry_bankroll_position_count,
        restored_entry_bankroll_accounted_pools = restored_entry_bankroll_accounted_pool_count,
        restored_entry_bankroll_spent_wei = %restored_entry_bankroll_spent_wei,
        restored_entry_bankroll_recovered_wei = %restored_entry_bankroll_recovered_wei,
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
                .mempool_signals(signal_limit, mempool_since_days)
                .await?;
            Ok::<_, eyre::Report>((status, pools, signals))
        }
        .await;
        let (status, pools, signals) = match poll_result {
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
                    poll_interval_ms,
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
        let pool_response_count = pools.count;
        let mut polled_pools = Vec::with_capacity(pools.pools.len());
        let mut polled_pool_wires = HashMap::with_capacity(pools.pools.len());
        for pool_wire in pools.pools {
            match pool_wire.to_pool_snapshot() {
                Ok(pool) => {
                    polled_pool_wires.insert(pool.address.clone(), pool_wire.clone());
                    polled_pools.push((pool_wire, pool));
                }
                Err(error) => {
                    warn!(error = %error, "skipping pool snapshot");
                }
            }
        }

        let mut market_events = 0usize;
        let mut risk_events = 0usize;
        let mut position_monitor_events = 0usize;
        let mut manual_close_requests = 0usize;
        let mut manual_close_failed = 0usize;
        let mut manual_close_reports = 0usize;
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
            if let Some(skip_reason) =
                mempool_signal_skip_reason_for_execution_mode(execution_mode, &signal.signal_type)
            {
                info!(
                    signal_id = %signal.signal_id,
                    signal_type = %signal.signal_type,
                    execution_mode = %execution_mode.label(),
                    reason = skip_reason,
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
                        "reason": skip_reason,
                        "execution_mode": execution_mode.label(),
                        "signal_type": signal.signal_type.as_str(),
                        "entry_path": "mined_pool_update",
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
            let projected_pool =
                prime_projected_mempool_entry_pool(&event, &pool_updates, &adapter_current_block);
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
        for (pool_wire, pool) in polled_pools {
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
            let mined_risk_candidates = mined_pool_risks_from_update(&pool_wire, &pool);
            let mined_risk_candidate_count = mined_risk_candidates.len();

            if suppress_events || (first_poll && !args.replay_current) {
                let mut primed_mined_risks = 0usize;
                for candidate in mined_risk_candidates {
                    if seen_mined_pool_risk_keys.insert(candidate.key.clone()) {
                        record_mined_pool_risk_observation(
                            &store,
                            &observation_strategy_name,
                            &candidate.key,
                            &candidate.event,
                            "primed",
                            0,
                            first_poll,
                            suppress_events,
                            &status,
                            json!({ "phase": "primed_from_pool_update" }),
                        )
                        .await?;
                        primed_mined_risks += 1;
                    }
                }
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
                    json!({
                        "mined_pool_risk_candidates": mined_risk_candidate_count,
                        "mined_pool_risks_primed": primed_mined_risks,
                    }),
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
                json!({
                    "reports": reports_payload(&event_reports),
                    "mined_pool_risk_candidates": mined_risk_candidate_count,
                }),
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

            for candidate in mined_risk_candidates {
                if !seen_mined_pool_risk_keys.insert(candidate.key.clone()) {
                    continue;
                }
                if let Some(block_number) = candidate.event.observed_block {
                    adapter_current_block.store(block_number, Ordering::Relaxed);
                }
                let event_reports = engine
                    .handle_event(EngineEvent::Risk(candidate.event.clone()))
                    .await?;
                let report_count = event_reports.len();
                let decision = if report_count > 0 {
                    "submitted"
                } else {
                    "hold"
                };
                record_mined_pool_risk_observation(
                    &store,
                    &observation_strategy_name,
                    &candidate.key,
                    &candidate.event,
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
                        risk_key = %candidate.key,
                        "chain-sim mined pool risk execution report"
                    );
                }
            }
        }

        if let (Some(vault_address), Some(manual_close_live_simulator)) = (
            manual_close_vault_address,
            manual_close_live_simulator.as_ref(),
        ) {
            if !suppress_events && (!first_poll || args.replay_current) {
                match process_manual_close_requests(
                    &store,
                    &mut engine,
                    manual_close_live_simulator,
                    vault_address,
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
                Ok(submitted) => match reconciler
                    .reconcile_after_processed_block(submitted, status.progress.current_block)
                    .await
                {
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
            token_server_pool_count = pool_response_count,
            signal_count = signals.count,
            market_events,
            risk_events,
            position_monitor_events,
            manual_close_requests,
            manual_close_failed,
            manual_close_reports,
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
            "token_server_pool_count": pool_response_count,
            "signal_count": signals.count,
            "market_events": market_events,
            "risk_events": risk_events,
            "position_monitor_events": position_monitor_events,
            "manual_close_requests": manual_close_requests,
            "manual_close_failed": manual_close_failed,
            "manual_close_reports": manual_close_reports,
            "receipt_reports": receipt_reports,
            "receipt_unresolved": receipt_unresolved,
            "reports": reports,
            "strategy_count": strategy_specs.len(),
            "observation_strategy_name": &observation_strategy_name,
            "positions": engine.portfolio().active_position_count(),
            "entry_enabled": !args.disable_entry,
            "max_entry_pools": single_max_entry_pools,
            "entry_bankroll_eth": &single_entry_bankroll_eth,
            "entry_bankroll_wei": single_entry_bankroll_wei.map(|value| value.to_string()),
            "entry_bankrolls": &entry_bankroll_summary,
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
        health.metrics.insert(
            "token_server_pool_count".to_string(),
            json!(pool_response_count),
        );
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
        health.metrics.insert(
            "manual_close_requests".to_string(),
            json!(manual_close_requests),
        );
        health.metrics.insert(
            "manual_close_failed".to_string(),
            json!(manual_close_failed),
        );
        health.metrics.insert(
            "manual_close_reports".to_string(),
            json!(manual_close_reports),
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
            _ = time::sleep(Duration::from_millis(poll_interval_ms)) => {}
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

fn mempool_signal_skip_reason_for_execution_mode(
    execution_mode: TraderExecutionMode,
    signal_type: &str,
) -> Option<&'static str> {
    (matches!(execution_mode, TraderExecutionMode::ChainSim) && signal_type == "trading_enabled")
        .then_some(CHAIN_SIM_SKIP_MEMPOOL_TRADING_ENABLED_REASON)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_sim_live_backtest_skips_trading_enabled_mempool_entries() {
        assert_eq!(
            mempool_signal_skip_reason_for_execution_mode(
                TraderExecutionMode::ChainSim,
                "trading_enabled"
            ),
            Some(CHAIN_SIM_SKIP_MEMPOOL_TRADING_ENABLED_REASON)
        );
        assert!(mempool_signal_skip_reason_for_execution_mode(
            TraderExecutionMode::ChainSim,
            "liquidity_removal"
        )
        .is_none());
    }

    #[test]
    fn kartal_real_keeps_trading_enabled_mempool_entries() {
        assert!(mempool_signal_skip_reason_for_execution_mode(
            TraderExecutionMode::KartalReal,
            "trading_enabled"
        )
        .is_none());
    }
}
