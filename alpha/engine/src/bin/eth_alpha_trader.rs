use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

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
use eth_alpha_engine::wire::{
    parse_address, LivePoolListResponse, LiveStatusResponse, MempoolSignalWire,
    MempoolSignalsResponse, PoolWire,
};
use eth_alpha_engine::{
    AlphaEngine, BlockCriticalRiskPolicy, EngineEvent, LiveChainSimExecutionAdapter,
};
use eth_alpha_store::{PostgresTradingStore, StrategyObservationRecord};
use eth_ops_events::{
    emit_health, emit_issue, JsonlOpsEventSink, MultiOpsEventSink, PipelineHealth,
    PipelineHealthStatus, PipelineImpact, PipelineIssue, PipelineSeverity, TracingOpsEventSink,
};
use eth_strategies::shared_rules::live::live_mempool_liquidity_removal_exit::{
    default_strategy_spec, observation_strategy_name, suite_specs, LiveStrategySpec,
    LiveStrategySpecOptions, STRATEGY_RUNTIME,
};
use eth_strategies::{LiveSnipeAllConfig, LiveSnipeAllStrategy, SnipeAllConfig};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::time;
use tracing::{info, warn};

const POOL_UPDATE_SOURCE: &str = "pool_update";
const MEMPOOL_SIGNAL_SOURCE: &str = "mempool_signal";
const POSITION_MONITOR_SOURCE: &str = "position_monitor";
const ALPHA_DATABASE_URL_CONFIG: &str = "ALPHA_DATABASE_URL";
const ALPHA_TRADER_LOG_DIR_CONFIG: &str = "ALPHA_TRADER_LOG_DIR";
const CHAIN_SERVER_BIND_CONFIG: &str = "CHAIN_SERVER_BIND";
const RETH_DATADIR_CONFIG: &str = "RETH_DATADIR";
const DEFAULT_ALPHA_TRADER_LOG_DIR: &str =
    "/home/nima/code/crypto/blockchains/eth/logs/alpha_trader";

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value_t = 2_000)]
    poll_interval_ms: u64,

    #[arg(long, default_value_t = 14)]
    mempool_since_days: i64,

    #[arg(long, default_value_t = 200)]
    signal_limit: i64,

    #[arg(long = "buy-wei", default_value = "10000000000000000")]
    buy_wei: String,

    #[arg(long, default_value = "0.5")]
    min_liquidity_eth: String,

    #[arg(long, default_value = "1000")]
    min_liquidity_usd: String,

    #[arg(long)]
    run_id: Option<String>,

    /// Execution mode. Only `chain-sim` is supported; theoretical fill modes are rejected.
    #[arg(long, default_value = "chain-sim")]
    mode: String,

    /// Process the current token-server snapshot immediately instead of only priming watermarks.
    #[arg(long, default_value_t = false)]
    replay_current: bool,

    #[arg(long, default_value_t = false)]
    once: bool,

    /// Max hold active pool-update blocks: force sell after this many distinct
    /// pool-update blocks while the position is open.
    /// Disabled by default.
    #[arg(long)]
    max_hold_blocks: Option<u64>,

    /// Stop-loss ratio: sell if price drops to this fraction of entry price.
    /// E.g., 0.7 = sell at -30% loss. Disabled by default.
    #[arg(long)]
    stop_loss_ratio: Option<String>,

    /// Take-profit ratio: sell if price rises to this multiple of entry price.
    /// E.g., 3.0 = sell at +200% profit. Disabled by default.
    #[arg(long)]
    take_profit_ratio: Option<String>,

    /// Retry failed exits after this many blocks. Disabled by default.
    #[arg(long)]
    exit_retry_interval_blocks: Option<u64>,

    /// Maximum failed exit reports before retry stops. Requires retry interval to matter.
    #[arg(long)]
    max_exit_retries: Option<u32>,

    /// Register a named strategy suite instead of the default single strategy.
    /// `mempool-live-exits` runs the live mempool liquidity-removal exit variants.
    #[arg(long)]
    strategy_suite: Option<String>,
}

fn build_strategy_specs(args: &Args) -> Result<Vec<LiveStrategySpec>> {
    let options = LiveStrategySpecOptions {
        stop_loss_ratio: args.stop_loss_ratio.clone(),
        take_profit_ratio: args.take_profit_ratio.clone(),
        max_hold_blocks: args.max_hold_blocks,
        exit_retry_interval_blocks: args.exit_retry_interval_blocks,
        max_exit_retries: args.max_exit_retries,
    };

    if let Some(suite) = args.strategy_suite.as_deref() {
        return suite_specs(suite, &options).map_err(|error| eyre!(error));
    }

    Ok(vec![default_strategy_spec(&options)])
}

fn live_strategy_spec_config_json(spec: &LiveStrategySpec) -> Value {
    json!({
        "strategy_name": spec.strategy_name,
        "strategy_impl": spec.strategy_impl,
        "strategy_label": spec.strategy_label,
        "strategy_runtime": STRATEGY_RUNTIME,
        "exit_liquidity_removal": spec.exit_liquidity_removal,
        "exit_tax": spec.exit_tax,
        "exit_lp_approval": spec.exit_lp_approval,
        "exit_lp_approval_critical_only": spec.exit_lp_approval_critical_only,
        "exit_scam": spec.exit_scam,
        "allowed_protocols": spec.allowed_protocols,
        "block_entry_on_lp_approval": spec.block_entry_on_lp_approval,
        "lp_approval_gate_min_pct": spec.lp_approval_gate_min_pct,
        "defer_buy_confirm_block_lp_approval_to_max_hold": spec.defer_buy_confirm_block_lp_approval_to_max_hold,
        "stop_loss_ratio": spec.stop_loss_ratio,
        "take_profit_ratio": spec.take_profit_ratio,
        "max_hold_blocks": spec.max_hold_blocks,
        "exit_retry_interval_blocks": spec.exit_retry_interval_blocks,
        "max_exit_retries": spec.max_exit_retries,
    })
}

fn release_stale_submitted_position(position: &mut Position) -> bool {
    match position.state {
        PositionState::SellSubmitted | PositionState::SellIntentCreated => {
            position.state = PositionState::BuyConfirmed;
            position.exit_order_id = None;
            position.exit_failure_reason = None;
            position.exit_retryable = true;
            true
        }
        PositionState::BuySubmitted | PositionState::BuyIntentCreated => {
            position.state = PositionState::BuyFailed;
            true
        }
        _ => false,
    }
}

#[derive(Clone)]
struct TokenServerClient {
    base_url: String,
    http: reqwest::Client,
}

impl TokenServerClient {
    fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    async fn status(&self) -> Result<LiveStatusResponse> {
        self.get_json("/eth/tokens/api/live/status").await
    }

    async fn pools(&self) -> Result<LivePoolListResponse> {
        self.get_json("/eth/tokens/api/live/pools").await
    }

    async fn mempool_signals(&self, limit: i64, since_days: i64) -> Result<MempoolSignalsResponse> {
        let path = format!("/eth/tokens/api/mempool/signals?limit={limit}&since_days={since_days}");
        self.get_json(&path).await
    }

    async fn get_json<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, path);
        self.http
            .get(&url)
            .send()
            .await
            .wrap_err_with(|| format!("request failed: {url}"))?
            .error_for_status()
            .wrap_err_with(|| format!("token server returned an error: {url}"))?
            .json::<T>()
            .await
            .wrap_err_with(|| format!("failed to decode token server response: {url}"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    let shared_config = load_shared_config()?;
    let execution_mode = normalize_execution_mode(&args.mode)?;
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
    if let Err(error) = init_alpha_trader_ops_events(&run_id, &shared_config) {
        warn!(error = %error, "failed to initialize alpha trader ops events");
    }

    let store = PostgresTradingStore::connect(&database_url, run_id.clone())
        .await
        .wrap_err("failed to initialize Postgres trading store")?;
    store
        .start_run(
            execution_mode,
            json!({
                "strategy_name": &observation_strategy_name,
                "strategy_impl": if strategy_specs.len() == 1 { strategy_specs[0].strategy_impl.clone() } else { "multi-strategy-live-suite".to_string() },
                "strategy_label": if strategy_specs.len() == 1 { strategy_specs[0].strategy_label.clone() } else { "Mempool Live Exit Suite".to_string() },
                "strategy_suite": args.strategy_suite.clone(),
                "strategy_count": strategy_specs.len(),
                "strategies": strategy_specs.iter().map(live_strategy_spec_config_json).collect::<Vec<_>>(),
                "strategy_runtime": STRATEGY_RUNTIME,
                "observation_strategy_name": &observation_strategy_name,
                "execution_model": "chain_state_evm_simulation",
                "token_server_url": &token_server_url,
                "reth_datadir": &reth_datadir,
                "poll_interval_ms": args.poll_interval_ms,
                "mempool_since_days": args.mempool_since_days,
                "signal_limit": args.signal_limit,
                "buy_wei": &args.buy_wei,
                "min_liquidity_eth": &args.min_liquidity_eth,
                "min_liquidity_usd": &args.min_liquidity_usd,
                "replay_current": args.replay_current,
                "exit_retry_interval_blocks": args.exit_retry_interval_blocks,
                "max_exit_retries": args.max_exit_retries,
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
    let adapter = LiveChainSimExecutionAdapter::with_prefix_and_next_order_sequence(
        live_simulator,
        tx_processor,
        run_id.clone(),
        next_order_sequence,
    )
    .wrap_err("failed to initialize chain-sim execution adapter")?;
    let adapter_current_block = adapter.current_block();
    let pool_updates = adapter.pools();
    let state_status_adapter = adapter.clone();

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
        let config = LiveSnipeAllConfig::new(SnipeAllConfig {
            strategy_name: StrategyName(spec.strategy_name.clone()),
            buy_amount: Amount {
                raw: buy_wei,
                decimals: 18,
            },
            sell_fraction: eth_alpha_core::amount::DecimalAmount::from(1),
            min_denom_reserve: min_liquidity_eth,
            min_stable_denom_reserve: min_liquidity_usd,
            stop_loss_ratio,
            take_profit_ratio,
            max_hold_blocks: spec.max_hold_blocks,
            exit_retry_interval_blocks: spec.exit_retry_interval_blocks,
            max_exit_retries: spec.max_exit_retries,
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
        engine.add_strategy(Box::new(LiveSnipeAllStrategy::with_bought_pools(
            config, seen_pools,
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
        mode = %execution_mode,
        strategy_suite = ?args.strategy_suite,
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
                    "eth_alpha_trader",
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
                    "eth_alpha_trader",
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
            reports,
            strategy_count = strategy_specs.len(),
            positions = engine.portfolio().active_position_count(),
            chain_sim_selected_block = chain_state_status.as_ref().ok().map(|state| state.selected_block_number),
            chain_sim_state_source = chain_state_status.as_ref().ok().map(|state| format!("{:?}", state.source)),
            "alpha trader tick"
        );
        let heartbeat_metadata = json!({
            "execution_model": "chain_state_evm_simulation",
            "chain_sim_state": chain_state_payload,
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
            "reports": reports,
            "strategy_count": strategy_specs.len(),
            "observation_strategy_name": &observation_strategy_name,
            "positions": engine.portfolio().active_position_count(),
        });
        let mut health = PipelineHealth::new(
            "eth_alpha_trader",
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

fn init_alpha_trader_ops_events(run_id: &str, config: &HashMap<String, String>) -> Result<()> {
    let root = optional_shared_config_value(config, ALPHA_TRADER_LOG_DIR_CONFIG)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ALPHA_TRADER_LOG_DIR));
    let run_dir = root.join(sanitize_path_segment(run_id));
    let sink = MultiOpsEventSink::new(vec![
        Arc::new(JsonlOpsEventSink::open(&run_dir)?),
        Arc::new(TracingOpsEventSink),
    ]);
    let initialized = eth_ops_events::init_global_sink(Arc::new(sink));
    info!(
        run_id,
        run_dir = %run_dir.display(),
        initialized,
        "initialized alpha trader ops events"
    );
    Ok(())
}

fn sanitize_path_segment(value: &str) -> String {
    let sanitized = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "alpha-trader".to_string()
    } else {
        sanitized
    }
}

async fn load_persisted_watermarks(
    store: &PostgresTradingStore,
    strategy_name: &str,
) -> Result<(HashMap<TokenPoolId, u64>, HashSet<String>)> {
    let cursors = store
        .load_strategy_observation_cursors(strategy_name)
        .await
        .wrap_err("failed to load alpha trader observation watermarks")?;
    let mut pool_blocks = HashMap::new();
    let mut signal_ids = HashSet::new();

    for cursor in cursors {
        match cursor.event_source.as_str() {
            POOL_UPDATE_SOURCE => {
                let Some(block_number) = cursor.block_number else {
                    continue;
                };
                let Some(pool_id) = cursor_pool_id(&cursor)? else {
                    continue;
                };
                pool_blocks
                    .entry(pool_id)
                    .and_modify(|current: &mut u64| *current = (*current).max(block_number))
                    .or_insert(block_number);
            }
            MEMPOOL_SIGNAL_SOURCE => {
                signal_ids.insert(cursor.event_key);
            }
            POSITION_MONITOR_SOURCE => {}
            _ => {}
        }
    }

    Ok((pool_blocks, signal_ids))
}

fn cursor_pool_id(
    cursor: &eth_alpha_store::StrategyObservationCursor,
) -> Result<Option<TokenPoolId>> {
    let Some(pool_identity) = cursor.pool_address.as_ref() else {
        return Ok(None);
    };
    if pool_identity.contains(':') {
        return Ok(Some(TokenPoolId::from(pool_identity.as_str())));
    }
    let Some(token_address) = cursor.token_address.as_ref() else {
        return Ok(None);
    };
    let token_address = parse_address(token_address)?;
    Ok(Some(TokenPoolId::new(token_address, pool_identity)))
}

async fn record_pool_observation(
    store: &PostgresTradingStore,
    strategy_name: &str,
    pool_wire: &PoolWire,
    pool: &PoolSnapshot,
    previous_block: Option<u64>,
    decision: &str,
    report_count: usize,
    first_poll: bool,
    suppress_events: bool,
    status: &LiveStatusResponse,
    extra: Value,
) -> Result<()> {
    store
        .record_strategy_observation(StrategyObservationRecord {
            strategy_name: strategy_name.to_string(),
            event_source: POOL_UPDATE_SOURCE.to_string(),
            event_key: format!("{}:{}", pool.address, pool.latest_block),
            token_address: Some(pool.token_address.to_string()),
            pool_address: Some(pool.address.to_string()),
            block_number: Some(pool.latest_block),
            event_timestamp: None,
            decision: decision.to_string(),
            report_count,
            payload: json!({
                "pool": pool_wire,
                "previous_block": previous_block,
                "latest_block": pool.latest_block,
                "first_poll": first_poll,
                "suppress_events": suppress_events,
                "live_status": status.progress.status,
                "live_current_block": status.progress.current_block,
                "live_blocks_processed": status.progress.blocks_processed,
                "live_warmup_total_blocks": status.progress.warmup_total_blocks,
                "extra": extra,
            }),
        })
        .await
        .wrap_err("failed to record pool strategy observation")
}

async fn record_position_monitor_observation(
    store: &PostgresTradingStore,
    strategy_name: &str,
    block_number: u64,
    decision: &str,
    report_count: usize,
    first_poll: bool,
    suppress_events: bool,
    status: &LiveStatusResponse,
    extra: Value,
) -> Result<()> {
    store
        .record_strategy_observation(StrategyObservationRecord {
            strategy_name: strategy_name.to_string(),
            event_source: POSITION_MONITOR_SOURCE.to_string(),
            event_key: format!("position_monitor:{block_number}"),
            token_address: None,
            pool_address: None,
            block_number: Some(block_number),
            event_timestamp: None,
            decision: decision.to_string(),
            report_count,
            payload: json!({
                "block_number": block_number,
                "first_poll": first_poll,
                "suppress_events": suppress_events,
                "live_status": status.progress.status,
                "live_current_block": status.progress.current_block,
                "live_blocks_processed": status.progress.blocks_processed,
                "live_warmup_total_blocks": status.progress.warmup_total_blocks,
                "extra": extra,
            }),
        })
        .await
        .wrap_err("failed to record position monitor strategy observation")
}

async fn record_signal_observation(
    store: &PostgresTradingStore,
    strategy_name: &str,
    signal: &MempoolSignalWire,
    decision: &str,
    report_count: usize,
    first_poll: bool,
    suppress_events: bool,
    status: &LiveStatusResponse,
    extra: Value,
) -> Result<()> {
    store
        .record_strategy_observation(StrategyObservationRecord {
            strategy_name: strategy_name.to_string(),
            event_source: MEMPOOL_SIGNAL_SOURCE.to_string(),
            event_key: signal.signal_id.clone(),
            token_address: signal.token_address.clone(),
            pool_address: signal.pool_address.clone(),
            block_number: status.progress.current_block,
            event_timestamp: signal.detection_timestamp.clone(),
            decision: decision.to_string(),
            report_count,
            payload: json!({
                "signal": signal,
                "first_poll": first_poll,
                "suppress_events": suppress_events,
                "live_status": status.progress.status,
                "live_current_block": status.progress.current_block,
                "live_blocks_processed": status.progress.blocks_processed,
                "live_warmup_total_blocks": status.progress.warmup_total_blocks,
                "extra": extra,
            }),
        })
        .await
        .wrap_err("failed to record mempool signal strategy observation")
}

fn reports_payload(reports: &[ExecutionReport]) -> Vec<Value> {
    reports
        .iter()
        .filter_map(|report| serde_json::to_value(report).ok())
        .collect()
}

fn normalize_execution_mode(mode: &str) -> Result<&'static str> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "chain-sim" | "chain_sim" | "chainsim" => Ok("chain-sim"),
        other => Err(eyre!(
            "unsupported execution mode {other:?}; only chain-sim is allowed"
        )),
    }
}

fn shared_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("config.env")
}

fn load_shared_config() -> Result<HashMap<String, String>> {
    let path = shared_config_path();
    let contents = fs::read_to_string(&path)
        .wrap_err_with(|| format!("failed to read shared config file {}", path.display()))?;
    Ok(parse_shared_config(&contents))
}

fn parse_shared_config(contents: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        values.insert(
            key.to_string(),
            unquote_config_value(value.trim()).to_string(),
        );
    }
    values
}

fn optional_shared_config_value(config: &HashMap<String, String>, key: &str) -> Option<String> {
    config
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn required_shared_config_value(config: &HashMap<String, String>, key: &str) -> Result<String> {
    optional_shared_config_value(config, key).ok_or_else(|| {
        eyre!(
            "{key} must be set in shared config file {}",
            shared_config_path().display()
        )
    })
}

fn chain_server_url_from_config(config: &HashMap<String, String>) -> Result<String> {
    let bind = required_shared_config_value(config, CHAIN_SERVER_BIND_CONFIG)?;
    Ok(format!("http://{bind}"))
}

fn unquote_config_value(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn parse_u256_decimal(value: &str) -> Result<U256> {
    U256::from_str_radix(value, 10).map_err(|error| eyre!("invalid decimal U256 {value}: {error}"))
}

fn resolve_database_url(config: &HashMap<String, String>) -> Result<String> {
    required_shared_config_value(config, ALPHA_DATABASE_URL_CONFIG)
}

fn default_run_id() -> String {
    let stamp = Utc::now().format("%Y%m%d-%H%M%SZ");
    format!("alpha-trader-{stamp}-pid-{}", std::process::id())
}

#[cfg(unix)]
struct ShutdownSignals {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl ShutdownSignals {
    fn new() -> Result<Self> {
        Ok(Self {
            interrupt: tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                .wrap_err("failed to install SIGINT handler")?,
            terminate: tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .wrap_err("failed to install SIGTERM handler")?,
        })
    }

    async fn recv(&mut self) {
        tokio::select! {
            _ = self.interrupt.recv() => {}
            _ = self.terminate.recv() => {}
        }
    }
}

#[cfg(not(unix))]
struct ShutdownSignals;

#[cfg(not(unix))]
impl ShutdownSignals {
    fn new() -> Result<Self> {
        Ok(Self)
    }

    async fn recv(&mut self) {
        let _ = tokio::signal::ctrl_c().await;
    }
}
