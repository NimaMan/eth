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
    risk::{RiskEvent, RiskKind},
    store::TradingStore,
    Strategy,
};
use eth_alpha_store::{PostgresTradingStore, StrategyObservationRecord};
use eth_live_trading::StrategyGasRankPolicy;
use eth_ops_events::{
    emit_health, emit_issue, JsonlOpsEventSink, MultiOpsEventSink, PipelineHealth,
    PipelineHealthStatus, PipelineImpact, PipelineIssue, PipelineSeverity, TracingOpsEventSink,
};
use eth_strategies::shared_rules::{
    entry::init_policy::EntryInitPolicyConfig,
    live::{
        default_strategy_spec, observation_strategy_name, strategy_set_specs, LiveStrategySpec,
        LiveStrategySpecOptions, STRATEGY_RUNTIME,
    },
};
use eth_strategies::{
    Alpha11Config, LiveAlpha11Config, LiveAlpha11Strategy, LiveSnipeAllConfig,
    LiveSnipeAllStrategy, RestoredEntryBankroll, SnipeAllConfig, ALPHA11_STRATEGY_IMPL,
};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use tokio::time;
use tracing::{info, warn};

mod backtest;
mod cli;
mod gas_policy;
mod manual_close;
mod position_state;
mod real_execution;
mod receipt_reconciliation;
mod strategy;
mod support;
mod token_server;

use backtest::ChainSimGasPolicyBacktestAdapter;
use cli::{parse_live_backtest_args, parse_live_real_args, Args, RealExecutionArgs};
use gas_policy::load_live_real_gas_policy;
use manual_close::{default_manual_close_limit, process_manual_close_requests};
use position_state::release_stale_submitted_position;
use real_execution::{build_kartal_real_adapter, preflight_kartal_real};
use receipt_reconciliation::{JsonRpcReceiptProvider, VaultReceiptReconciler};
use strategy::{build_strategy_specs, live_strategy_spec_config_json};
use support::*;
use token_server::TokenServerClient;

const POOL_UPDATE_SOURCE: &str = "pool_update";
const MEMPOOL_SIGNAL_SOURCE: &str = "mempool_signal";
const POSITION_MONITOR_SOURCE: &str = "position_monitor";
const ALPHA_DATABASE_CONFIG_KEY: &str = "databases.alpha.url";
const ALPHA_TRADER_LOG_DIR_CONFIG: &str = "ALPHA_TRADER_LOG_DIR";
const ALPHA_LIVE_TRADER_POLL_INTERVAL_MS_CONFIG: &str = "ALPHA_LIVE_TRADER_POLL_INTERVAL_MS";
const ALPHA_LIVE_MEMPOOL_SINCE_DAYS_CONFIG: &str = "ALPHA_LIVE_MEMPOOL_SINCE_DAYS";
const ALPHA_LIVE_SIGNAL_LIMIT_CONFIG: &str = "ALPHA_LIVE_SIGNAL_LIMIT";
const CHAIN_SERVER_BIND_CONFIG: &str = "CHAIN_SERVER_BIND";
const RETH_DATADIR_CONFIG: &str = "RETH_DATADIR";
const DEFAULT_ALPHA_TRADER_LOG_DIR: &str =
    "/home/nima/code/crypto/blockchains/eth/logs/alpha_trader";
const DEFAULT_KARTAL_URL: &str = "http://127.0.0.1:5004";
const DEFAULT_KARTAL_TOKEN_ENV: &str = "ETH_TX_EXECUTOR_API_TOKEN";
const DEFAULT_LIVE_REAL_FROM: &str = "0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27";
const DEFAULT_UNISWAP_V2_TRADING_VAULT: &str = "0x28474cbCd780AeEb3ED1501B68254bEd87cF5597";
const LIVE_REAL_VALIDATION_MAX_ENTRY_BANKROLL_ETH: &str = "0.555";
const MAX_LIVE_TRADER_POLL_INTERVAL_MS: u64 = 1_000;

fn resolve_entry_bankroll_wei(spec: &LiveStrategySpec) -> Result<Option<U256>> {
    let Some(value) = spec.entry_bankroll_eth.as_deref() else {
        return Ok(None);
    };
    let label = format!("strategy {} entry_bankroll_eth", spec.strategy_name);
    parse_eth_decimal_to_wei(value, &label).map(Some)
}

fn entry_bankroll_summary_json(
    specs: &[LiveStrategySpec],
    bankrolls_wei: &[Option<U256>],
) -> Vec<Value> {
    specs
        .iter()
        .zip(bankrolls_wei.iter())
        .map(|(spec, bankroll_wei)| {
            let source = if spec.entry_bankroll_eth.is_some() {
                "strategy_spec"
            } else {
                "none"
            };
            json!({
                "strategy_name": &spec.strategy_name,
                "entry_bankroll_eth": spec.entry_bankroll_eth.as_deref(),
                "entry_bankroll_wei": bankroll_wei.as_ref().map(|value| value.to_string()),
                "source": source,
            })
        })
        .collect()
}

fn position_entry_spend_wei(position: &Position, fallback_buy_wei: U256) -> U256 {
    position
        .entry_cost_basis
        .map(|cost| Amount::from_decimal(cost, 18).raw)
        .unwrap_or(fallback_buy_wei)
}

fn position_exit_proceeds_wei(position: &Position) -> U256 {
    position
        .exit_proceeds
        .map(|proceeds| Amount::from_decimal(proceeds, 18).raw)
        .unwrap_or(U256::ZERO)
}

fn restored_entry_bankroll_from_terminal_positions(
    positions: &[Position],
    fallback_buy_wei: U256,
) -> RestoredEntryBankroll {
    let mut bankroll = RestoredEntryBankroll::default();
    for position in positions {
        match position.state {
            PositionState::BuyFailed | PositionState::BuyCancelled | PositionState::Cancelled => {
                bankroll.record_accounted_pool(position.key.pool_address.clone());
            }
            PositionState::SellConfirmed => {
                bankroll.record_position_result(
                    position.key.pool_address.clone(),
                    position_entry_spend_wei(position, fallback_buy_wei),
                    position_exit_proceeds_wei(position),
                );
            }
            PositionState::Scammed => {
                bankroll.record_position_result(
                    position.key.pool_address.clone(),
                    position_entry_spend_wei(position, fallback_buy_wei),
                    U256::ZERO,
                );
            }
            PositionState::Init
            | PositionState::BuyIntentCreated
            | PositionState::BuySubmitted
            | PositionState::BuyDeferred
            | PositionState::BuyConfirmed
            | PositionState::SellIntentCreated
            | PositionState::SellSubmitted
            | PositionState::SellFailed
            | PositionState::SellCancelled => {}
        }
    }
    bankroll
}

fn single_strategy_value<T>(
    specs: &[LiveStrategySpec],
    value: impl FnOnce(&LiveStrategySpec) -> T,
) -> Option<T> {
    if specs.len() == 1 {
        Some(value(&specs[0]))
    } else {
        None
    }
}

fn live_gas_policy_run_metadata_json(
    policy: &gas_policy::LiveRealGasPolicy,
    execution_mode: TraderExecutionMode,
) -> Value {
    json!({
        "mode": if execution_mode.uses_kartal() { "kartal-real" } else { "chain-sim-shadow" },
        "required_gas_rank_source": &policy.required_gas_rank_source,
        "gas_rank_lookback_blocks": policy.gas_rank_lookback_blocks,
        "simulated_gas_buffer_bps": policy.simulated_gas_buffer_bps,
        "max_priority_fee_gwei": policy.max_priority_fee_gwei.to_string(),
        "entry_max_estimated_gas_fee_eth": policy.entry_max_estimated_gas_fee_eth.to_string(),
        "exit_max_estimated_gas_fee_eth": policy.exit_max_estimated_gas_fee_eth.to_string(),
        "safety_buffer_eth": policy.safety_buffer_eth.to_string(),
        "v2_vault_buy_gas_limit": policy.v2_vault_buy_gas_limit,
        "v2_vault_sell_gas_limit": policy.v2_vault_sell_gas_limit,
        "entry_buy_profiles": &policy.entry_buy_gas_rank_policy,
        "normal_exit_profiles": &policy.normal_exit_gas_rank_policy,
        "mempool_race_exit_profiles": &policy.mempool_pre_mine_gas_rank_policy,
        "lp_approval_exit_profiles": &policy.lp_approval_exit_gas_rank_policy,
    })
}

fn resolve_cli_or_config_u64(
    override_value: Option<u64>,
    config: &HashMap<String, String>,
    key: &str,
) -> Result<u64> {
    if let Some(value) = override_value {
        return Ok(value);
    }
    let value = required_shared_config_value(config, key)?;
    value
        .parse::<u64>()
        .wrap_err_with(|| format!("invalid {key} value {value:?}"))
}

fn resolve_cli_or_config_i64(
    override_value: Option<i64>,
    config: &HashMap<String, String>,
    key: &str,
) -> Result<i64> {
    if let Some(value) = override_value {
        return Ok(value);
    }
    let value = required_shared_config_value(config, key)?;
    value
        .parse::<i64>()
        .wrap_err_with(|| format!("invalid {key} value {value:?}"))
}

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
    let strategy_specs = build_strategy_specs(&args, execution_mode)?;
    let mut live_gas_policy = load_live_real_gas_policy(&shared_config)?;
    if execution_mode.uses_kartal() {
        live_gas_policy.mempool_pre_mine_gas_rank_policy =
            StrategyGasRankPolicy::mempool_race_only();
    }
    let live_real_gas_policy = if execution_mode.uses_kartal() {
        Some(live_gas_policy.clone())
    } else {
        None
    };
    let mut kartal_real_preflight = match real_args.as_ref() {
        None => None,
        Some(real_args) => Some(preflight_kartal_real(real_args, &args, &strategy_specs).await?),
    };
    let token_server_url = chain_server_url_from_config(&shared_config)?;
    let reth_datadir = required_shared_config_value(&shared_config, RETH_DATADIR_CONFIG)?;
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
                        "allow_public_mempool_live_validation": real_args.allow_public_mempool_live_validation
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
    let mut portfolio = PortfolioState::default();
    let mut seen_pools_by_strategy = HashMap::new();
    let mut active_hold_counters_by_strategy = HashMap::new();
    let mut restored_entry_bankrolls_by_strategy = HashMap::new();
    let mut restored_entry_bankroll_position_count = 0usize;
    let mut restored_stale_submitted_positions = 0usize;
    for spec in &strategy_specs {
        let buy_wei = parse_u256_decimal(&spec.buy_wei).wrap_err_with(|| {
            format!(
                "invalid buy_wei for live strategy {}: {}",
                spec.strategy_name, spec.buy_wei
            )
        })?;
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

        let terminal_positions = store
            .load_terminal_positions(&spec.strategy_name)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to restore terminal alpha positions for {}",
                    spec.strategy_name
                )
            })?;
        restored_entry_bankroll_position_count += terminal_positions.len();
        let restored_entry_bankroll =
            restored_entry_bankroll_from_terminal_positions(&terminal_positions, buy_wei);
        restored_entry_bankrolls_by_strategy
            .insert(spec.strategy_name.clone(), restored_entry_bankroll);

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
    let restored_entry_bankroll_accounted_pool_count = restored_entry_bankrolls_by_strategy
        .values()
        .map(RestoredEntryBankroll::accounted_pool_count)
        .sum::<usize>();
    let restored_entry_bankroll_spent_wei = restored_entry_bankrolls_by_strategy
        .values()
        .fold(U256::ZERO, |acc, bankroll| {
            acc.saturating_add(bankroll.spent_wei())
        });
    let restored_entry_bankroll_recovered_wei = restored_entry_bankrolls_by_strategy
        .values()
        .fold(U256::ZERO, |acc, bankroll| {
            acc.saturating_add(bankroll.recovered_wei())
        });

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
    let exact_pre_submit_live_simulator = chain_sim_adapter.live_simulator();
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
        let buy_wei = parse_u256_decimal(&spec.buy_wei).wrap_err_with(|| {
            format!(
                "invalid buy_wei for live strategy {}: {}",
                spec.strategy_name, spec.buy_wei
            )
        })?;
        let min_liquidity_eth = Decimal::from_str(&spec.min_liquidity_eth).wrap_err_with(|| {
            format!(
                "invalid min_liquidity_eth for live strategy {}: {}",
                spec.strategy_name, spec.min_liquidity_eth
            )
        })?;
        let min_liquidity_usd = Decimal::from_str(&spec.min_liquidity_usd).wrap_err_with(|| {
            format!(
                "invalid min_liquidity_usd for live strategy {}: {}",
                spec.strategy_name, spec.min_liquidity_usd
            )
        })?;
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
        let entry_init_max_price_ratio_to_initial = spec
            .entry_init_policy
            .max_price_ratio_to_initial
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok());
        let entry_init_policy = EntryInitPolicyConfig {
            max_age_blocks: spec.entry_init_policy.max_age_blocks,
            require_creation_block: spec.entry_init_policy.require_creation_block,
            max_price_ratio_to_initial: entry_init_max_price_ratio_to_initial,
            allow_missing_price_ratio: spec.entry_init_policy.allow_missing_price_ratio,
        };
        let min_sell_pool_denom_reserve = spec
            .min_sell_pool_denom_reserve
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or_else(|| SnipeAllConfig::default().min_sell_pool_denom_reserve);
        let snipe_all_config = SnipeAllConfig {
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
            max_entry_pools: spec.max_entry_pools,
            entry_bankroll_wei,
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
            entry_init_policy,
            defer_buy_confirm_block_lp_approval_to_max_hold: spec
                .defer_buy_confirm_block_lp_approval_to_max_hold,
            lp_approval_exit_defer_max_trading_enabled_age_blocks: spec
                .lp_approval_exit_defer_max_trading_enabled_age_blocks,
            ..SnipeAllConfig::default()
        };
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
        let strategy: Box<dyn Strategy> = match spec.strategy_impl.as_str() {
            ALPHA11_STRATEGY_IMPL => Box::new(LiveAlpha11Strategy::with_restored_runtime_state(
                LiveAlpha11Config::new(Alpha11Config::new(snipe_all_config)),
                seen_pools,
                active_hold_counters,
                restored_entry_bankroll,
            )),
            "snipe-all" => Box::new(LiveSnipeAllStrategy::with_restored_runtime_state(
                LiveSnipeAllConfig::new(snipe_all_config),
                seen_pools,
                active_hold_counters,
                restored_entry_bankroll,
            )),
            other => {
                return Err(eyre!(
                    "{} cannot instantiate unsupported live strategy_impl {} for {}",
                    runner_name,
                    other,
                    spec.strategy_name
                ));
            }
        };
        engine.add_strategy(strategy);
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
        max_entry_pools = ?single_max_entry_pools,
        entry_bankroll_eth = ?single_entry_bankroll_eth,
        entry_bankroll_wei = ?single_entry_bankroll_wei.map(|value| value.to_string()),
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

        if let Some(vault_address) = manual_close_vault_address {
            if !suppress_events && (!first_poll || args.replay_current) {
                match process_manual_close_requests(
                    &store,
                    &mut engine,
                    &manual_close_live_simulator,
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

fn annotate_signal_risk_event(
    event: &mut RiskEvent,
    signal: &MempoolSignalWire,
    pool: Option<&PoolWire>,
) {
    let mut evidence = event
        .evidence
        .take()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    evidence.insert("signal_id".to_string(), json!(signal.signal_id));
    evidence.insert("signal_type".to_string(), json!(signal.signal_type));
    if let Some(value) = signal.signal_source.as_ref() {
        evidence.insert("signal_source".to_string(), json!(value));
    }
    if let Some(value) = event.observed_block {
        evidence.insert("observed_block".to_string(), json!(value));
    }
    if event.kind == RiskKind::LpApproval {
        annotate_lp_approval_age_evidence(&mut evidence, event, pool);
    }
    event.evidence = Some(Value::Object(evidence));
}

fn annotate_lp_approval_age_evidence(
    evidence: &mut Map<String, Value>,
    event: &RiskEvent,
    pool: Option<&PoolWire>,
) {
    let Some(observed_block) = event.observed_block else {
        evidence.insert("lp_approval_age_basis".to_string(), json!("unknown"));
        return;
    };
    let Some(pool) = pool else {
        evidence.insert(
            "lp_approval_age_basis".to_string(),
            json!("missing_pool_context"),
        );
        return;
    };
    if let Some(can_buy_block) = pool.can_buy_block {
        evidence.insert("trading_enabled_block".to_string(), json!(can_buy_block));
        evidence.insert(
            "trading_enabled_age_blocks_at_signal".to_string(),
            json!(observed_block as i64 - can_buy_block as i64),
        );
        evidence.insert(
            "lp_approval_age_basis".to_string(),
            json!("trading_enabled_block"),
        );
    }
    if let Some(creation_block) = pool.creation_block {
        evidence.insert("pool_creation_block".to_string(), json!(creation_block));
        evidence.insert(
            "pool_age_blocks_at_signal".to_string(),
            json!(observed_block as i64 - creation_block as i64),
        );
        if !evidence.contains_key("lp_approval_age_basis") {
            evidence.insert(
                "lp_approval_age_basis".to_string(),
                json!("pool_creation_block"),
            );
        }
    }
    if !evidence.contains_key("lp_approval_age_basis") {
        evidence.insert("lp_approval_age_basis".to_string(), json!("unknown"));
    }
}
