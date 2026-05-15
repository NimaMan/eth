use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use eth_alpha_backtest::{adapter::BacktestAdapter, runner::run_backtest};
use eth_alpha_core::amount::Amount;
use eth_alpha_core::ids::{PoolAddress, StrategyName};
use eth_alpha_core::market::PoolSnapshot;
use eth_alpha_engine::wire::PoolWire;
use eth_alpha_engine::{AlphaEngine, BlockCriticalRiskPolicy};
use eth_alpha_store::PostgresTradingStore;
use eth_strategies::{SnipeAllConfig, SnipeAllStrategy};
use eyre::{Result, WrapErr};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::Row;
use tracing::info;

#[derive(Debug, Parser)]
struct Args {
    /// Unique run identifier for this backtest.
    #[arg(long)]
    run_id: Option<String>,

    /// Strategy instance name to persist on orders, positions, and reports.
    #[arg(long, default_value = "snipe-all-v1")]
    strategy_name: String,

    /// Strategy implementation to instantiate.
    #[arg(long, default_value = "snipe-all-v1")]
    strategy_impl: String,

    /// Named historical strategy suite to run in one replay pass.
    #[arg(long)]
    strategy_suite: Option<String>,

    /// Existing live chain-sim run_id to replay from `strategy_observations`.
    #[arg(long)]
    replay_run_id: String,

    /// Buy amount in wei (also used as sell amount for snipe-all).
    #[arg(long, default_value = "10000000000000000")]
    buy_amount_wei: String,

    /// Minimum ETH/WETH reserve for a pool to be eligible.
    #[arg(long, default_value = "0.5")]
    min_liquidity_eth: String,

    /// Minimum stable-denom reserve (USDC/USDT/DAI) for eligibility.
    #[arg(long, default_value = "1000")]
    min_liquidity_usd: String,

    /// Skip primed/warmup observations (suppress_events=true).
    /// Live trader skips these; enabling makes backtest apples-to-apples.
    #[arg(long, default_value_t = false)]
    skip_primed: bool,

    /// Start block (inclusive). If not set, starts from first observation.
    #[arg(long)]
    from_block: Option<u64>,

    /// End block (inclusive). If not set, runs to last observation.
    #[arg(long)]
    to_block: Option<u64>,

    /// Stop-loss ratio: sell if price drops to this fraction of entry price.
    /// E.g., 0.7 = sell at -30% loss. Disabled by default.
    #[arg(long)]
    stop_loss_ratio: Option<String>,

    /// Take-profit ratio: sell if price rises to this multiple of entry price.
    /// E.g., 3.0 = sell at +200% profit. Disabled by default.
    #[arg(long)]
    take_profit_ratio: Option<String>,

    /// Max hold blocks: force sell after this many blocks regardless of price.
    /// Disabled by default.
    #[arg(long)]
    max_hold_blocks: Option<u64>,

    /// Retry failed exits after this many blocks. Disabled by default.
    #[arg(long)]
    exit_retry_interval_blocks: Option<u64>,

    /// Maximum failed exit reports before retry stops. Requires retry interval to matter.
    #[arg(long)]
    max_exit_retries: Option<u32>,

    /// Blocks between signal observation/submission and simulated confirmation.
    /// Default 1 means observe N, submit at N, fill against post-block N+1 state.
    #[arg(long, default_value_t = 1)]
    execution_delay_blocks: u64,
}

#[derive(Clone, Debug)]
struct BacktestStrategySpec {
    strategy_name: String,
    strategy_impl: String,
    stop_loss_ratio: Option<String>,
    take_profit_ratio: Option<String>,
    max_hold_blocks: Option<u64>,
    exit_retry_interval_blocks: Option<u64>,
    max_exit_retries: Option<u32>,
}

impl BacktestStrategySpec {
    fn config_json(&self) -> Value {
        serde_json::json!({
            "strategy_name": self.strategy_name,
            "strategy_impl": self.strategy_impl,
            "stop_loss_ratio": self.stop_loss_ratio,
            "take_profit_ratio": self.take_profit_ratio,
            "max_hold_blocks": self.max_hold_blocks,
            "exit_retry_interval_blocks": self.exit_retry_interval_blocks,
            "max_exit_retries": self.max_exit_retries,
        })
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
    let database_url = required_shared_config_value(&shared_config, "ALPHA_DATABASE_URL")?;
    let reth_datadir = required_shared_config_value(&shared_config, "RETH_DATADIR")?;

    let run_id = args.run_id.clone().unwrap_or_else(default_run_id);
    let strategy_specs = build_strategy_specs(&args)?;

    let buy_amount = Amount {
        raw: alloy_primitives::U256::from_str_radix(&args.buy_amount_wei, 10)
            .wrap_err("invalid --buy-amount-wei")?,
        decimals: 18,
    };
    let min_liquidity_eth =
        Decimal::from_str(&args.min_liquidity_eth).wrap_err("invalid --min-liquidity-eth")?;
    let min_liquidity_usd =
        Decimal::from_str(&args.min_liquidity_usd).wrap_err("invalid --min-liquidity-usd")?;

    let store = PostgresTradingStore::connect(&database_url, run_id.clone())
        .await
        .wrap_err("failed to connect to Postgres trading store")?;

    store
        .start_run(
            "backtest",
            serde_json::json!({
                "strategy_name": args.strategy_name,
                "strategy_impl": args.strategy_impl,
                "strategy_runtime": "historical",
                "strategy_input_timing": "confirmed-history",
                "strategy_suite": args.strategy_suite,
                "strategies": strategy_specs.iter().map(BacktestStrategySpec::config_json).collect::<Vec<_>>(),
                "replay_run_id": args.replay_run_id,
                "from_block": args.from_block,
                "to_block": args.to_block,
                "skip_primed": args.skip_primed,
                "buy_amount_wei": args.buy_amount_wei,
                "min_liquidity_eth": min_liquidity_eth.to_string(),
                "min_liquidity_usd": min_liquidity_usd.to_string(),
                "stop_loss_ratio": args.stop_loss_ratio,
                "take_profit_ratio": args.take_profit_ratio,
                "max_hold_blocks": args.max_hold_blocks,
                "exit_retry_interval_blocks": args.exit_retry_interval_blocks,
                "max_exit_retries": args.max_exit_retries,
                "execution_delay_blocks": args.execution_delay_blocks,
            }),
        )
        .await
        .wrap_err("failed to start backtest run")?;

    let events = load_events_from_observations(
        store.pool(),
        &args.replay_run_id,
        args.skip_primed,
        args.from_block,
        args.to_block,
    )
    .await
    .wrap_err_with(|| format!("failed to load observations for run {}", args.replay_run_id))?;

    if events.is_empty() {
        return Err(eyre::eyre!(
            "no valid events found for replay run {}",
            args.replay_run_id
        ));
    }

    info!(reth_datadir = %reth_datadir, "initialising chain-sim execution adapter");
    let simulator = Arc::new(tx_simulator::TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(tx_processor::tx_processor::TxProcessor::new());
    let inner = eth_alpha_engine::execution::ChainSimExecutionAdapter::with_prefix(
        simulator,
        tx_processor,
        run_id.clone(),
    )?
    .with_execution_delay_blocks(args.execution_delay_blocks);
    let adapter = ChainSimBacktestAdapter::new(inner);

    run_backtest_with_adapter(
        args,
        store,
        events,
        adapter,
        run_id,
        strategy_specs,
        buy_amount,
        min_liquidity_eth,
        min_liquidity_usd,
    )
    .await?;

    Ok(())
}

/// Load historical events by replaying `strategy_observations` from an existing
/// live chain-sim run.
async fn load_events_from_observations(
    pool: &sqlx::PgPool,
    replay_run_id: &str,
    skip_primed: bool,
    from_block: Option<u64>,
    to_block: Option<u64>,
) -> Result<Vec<eth_alpha_engine::EngineEvent>> {
    let replay_block_expr =
        "COALESCE(block_number, NULLIF(payload->>'live_current_block', '')::BIGINT)";
    let mut query = format!(
        "SELECT event_source, event_key, block_number, payload, {replay_block_expr} AS replay_block
         FROM alpha_trading.strategy_observations
         WHERE run_id = $1",
    );
    if from_block.is_some() {
        query.push_str(&format!(" AND {replay_block_expr} >= $2"));
    }
    if to_block.is_some() {
        query.push_str(&format!(
            " AND {replay_block_expr} <= ${}",
            if from_block.is_some() { 3 } else { 2 }
        ));
    }
    query.push_str(&format!(
        " ORDER BY {replay_block_expr} ASC NULLS LAST, first_seen_at ASC"
    ));

    let mut q = sqlx::query(&query).bind(replay_run_id);
    if let Some(b) = from_block {
        q = q.bind(b as i64);
    }
    if let Some(b) = to_block {
        q = q.bind(b as i64);
    }
    let rows = q
        .fetch_all(pool)
        .await
        .wrap_err("failed to query strategy_observations")?;

    let mut events = Vec::with_capacity(rows.len());
    let mut skipped = 0usize;
    let mut skipped_mempool = 0usize;

    for row in &rows {
        let event_source: String = row.try_get("event_source")?;
        let payload: Value = row.try_get("payload")?;
        if skip_primed && payload.get("suppress_events").and_then(|v| v.as_bool()) == Some(true) {
            skipped += 1;
            continue;
        }

        match event_source.as_str() {
            "pool_update" => {
                let mut pool_wire: PoolWire = match serde_json::from_value(
                    payload.get("pool").cloned().unwrap_or(Value::Null),
                ) {
                    Ok(w) => w,
                    Err(error) => {
                        tracing::warn!(error = %error, "skipping malformed pool observation");
                        skipped += 1;
                        continue;
                    }
                };
                // Historical observations recorded before May 2026 may be missing
                // denom_symbol / denom_address. Default to WETH for backtest
                // validation so classification does not reject every pool.
                if pool_wire.denom_symbol.is_none() && pool_wire.currency.is_none() {
                    pool_wire.denom_symbol = Some("WETH".to_string());
                }
                let pool = match pool_wire.to_pool_snapshot() {
                    Ok(p) => p,
                    Err(error) => {
                        tracing::warn!(error = %error, "skipping pool with missing fields");
                        skipped += 1;
                        continue;
                    }
                };
                events.push(eth_alpha_engine::EngineEvent::Market(
                    eth_alpha_core::market::MarketEvent::PoolUpdated {
                        block_number: pool.latest_block,
                        pool,
                    },
                ));
            }
            "mempool_signal" => {
                skipped_mempool += 1;
                continue;
            }
            other => {
                tracing::warn!(event_source = %other, "skipping unknown observation source");
                skipped += 1;
            }
        }
    }

    if skipped > 0 || skipped_mempool > 0 {
        tracing::info!(
            skipped_malformed = skipped,
            skipped_mempool,
            total = rows.len(),
            "skipped historical observations"
        );
    }

    Ok(add_block_completed_events(events, from_block, to_block))
}

fn add_block_completed_events(
    events: Vec<eth_alpha_engine::EngineEvent>,
    from_block: Option<u64>,
    to_block: Option<u64>,
) -> Vec<eth_alpha_engine::EngineEvent> {
    let mut by_block: BTreeMap<u64, Vec<eth_alpha_engine::EngineEvent>> = BTreeMap::new();
    let mut without_block = Vec::new();

    for event in events {
        if let Some(block) = event_block(&event) {
            by_block.entry(block).or_default().push(event);
        } else {
            without_block.push(event);
        }
    }

    let Some(first_event_block) = by_block.keys().next().copied() else {
        return without_block;
    };
    let Some(last_event_block) = by_block.keys().next_back().copied() else {
        return without_block;
    };
    let start_block = from_block.unwrap_or(first_event_block);
    let end_block = to_block.unwrap_or(last_event_block);
    if start_block > end_block {
        return without_block;
    }

    let mut expanded = Vec::with_capacity(
        without_block
            .len()
            .saturating_add(by_block.values().map(Vec::len).sum())
            .saturating_add((end_block - start_block + 1) as usize),
    );
    for block in start_block..=end_block {
        let mut updated_pools = 0usize;
        if let Some(block_events) = by_block.remove(&block) {
            updated_pools = block_events
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        eth_alpha_engine::EngineEvent::Market(
                            eth_alpha_core::market::MarketEvent::PoolUpdated { .. }
                        )
                    )
                })
                .count();
            expanded.extend(block_events);
        }
        expanded.push(eth_alpha_engine::EngineEvent::Market(
            eth_alpha_core::market::MarketEvent::BlockCompleted {
                block_number: block,
                updated_tokens: 0,
                updated_pools,
            },
        ));
    }
    expanded.extend(without_block);
    expanded
}

fn event_block(event: &eth_alpha_engine::EngineEvent) -> Option<u64> {
    match event {
        eth_alpha_engine::EngineEvent::Market(
            eth_alpha_core::market::MarketEvent::PoolUpdated { block_number, .. }
            | eth_alpha_core::market::MarketEvent::TokenUpdated { block_number, .. }
            | eth_alpha_core::market::MarketEvent::BlockCompleted { block_number, .. },
        ) => Some(*block_number),
        eth_alpha_engine::EngineEvent::Risk(risk) => risk.observed_block,
        eth_alpha_engine::EngineEvent::Execution(report) => report.block_number,
    }
}

#[derive(Clone)]
struct ChainSimBacktestAdapter(eth_alpha_engine::execution::ChainSimExecutionAdapter);

impl ChainSimBacktestAdapter {
    fn new(inner: eth_alpha_engine::execution::ChainSimExecutionAdapter) -> Self {
        Self(inner)
    }
}

#[async_trait::async_trait]
impl eth_alpha_engine::EngineExecutionAdapter for ChainSimBacktestAdapter {
    async fn execute(
        &self,
        intent: eth_alpha_core::order::OrderIntent,
    ) -> eth_alpha_core::error::Result<eth_alpha_core::execution::ExecutionReport> {
        self.0.execute(intent).await
    }

    async fn simulate_position_value(
        &self,
        position: &eth_alpha_core::position::Position,
        pool: &PoolSnapshot,
    ) -> eth_alpha_core::error::Result<Option<eth_alpha_engine::PositionValueSimulation>> {
        self.0.simulate_position_value(position, pool).await
    }
}

impl BacktestAdapter for ChainSimBacktestAdapter {
    fn pools(&self) -> Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>> {
        self.0.pools()
    }

    fn current_block(&self) -> Arc<AtomicU64> {
        self.0.current_block()
    }
}

async fn run_backtest_with_adapter<A>(
    args: Args,
    store: PostgresTradingStore,
    events: Vec<eth_alpha_engine::EngineEvent>,
    adapter: A,
    run_id: String,
    strategy_specs: Vec<BacktestStrategySpec>,
    buy_amount: Amount,
    min_liquidity_eth: Decimal,
    min_liquidity_usd: Decimal,
) -> Result<()>
where
    A: eth_alpha_engine::EngineExecutionAdapter + BacktestAdapter + Clone,
{
    let mut engine = AlphaEngine::new(BlockCriticalRiskPolicy, store.clone(), adapter.clone());

    for spec in &strategy_specs {
        match spec.strategy_impl.as_str() {
            "snipe-all-v1" => {
                let stop_loss_ratio = spec
                    .stop_loss_ratio
                    .as_deref()
                    .and_then(|s| Decimal::from_str(s).ok());
                let take_profit_ratio = spec
                    .take_profit_ratio
                    .as_deref()
                    .and_then(|s| Decimal::from_str(s).ok());

                engine.add_strategy(Box::new(SnipeAllStrategy::new(SnipeAllConfig {
                    strategy_name: StrategyName(spec.strategy_name.clone()),
                    buy_amount: buy_amount.clone(),
                    sell_fraction: eth_alpha_core::amount::DecimalAmount::from(1),
                    min_denom_reserve: min_liquidity_eth,
                    min_stable_denom_reserve: min_liquidity_usd,
                    exit_on_liquidity_removal: false,
                    exit_on_tax: false,
                    exit_on_lp_approval: false,
                    exit_on_critical_lp_approval_only: false,
                    exit_on_scam: false,
                    stop_loss_ratio,
                    take_profit_ratio,
                    max_hold_blocks: spec.max_hold_blocks,
                    exit_retry_interval_blocks: spec.exit_retry_interval_blocks,
                    max_exit_retries: spec.max_exit_retries,
                    ..SnipeAllConfig::default()
                })));
            }
            other => {
                return Err(eyre::eyre!("unsupported strategy implementation: {other}"));
            }
        }
    }

    if strategy_specs.is_empty() {
        return Err(eyre::eyre!("no strategies configured"));
    }

    info!(
        run_id = %run_id,
        replay_run_id = %args.replay_run_id,
        event_count = events.len(),
        strategy_count = strategy_specs.len(),
        strategy_suite = ?args.strategy_suite,
        "starting backtest"
    );

    let result = run_backtest(&mut engine, &adapter, events).await?;

    let total_positions = engine.portfolio().positions.len();
    let open_positions = engine.portfolio().active_position_count();

    store
        .mark_stopped(
            "completed",
            serde_json::json!({
                "events_processed": result.events_processed,
                "reports_generated": result.reports_generated,
                "confirmed_reports": result.confirmed_reports,
                "failed_reports": result.failed_reports,
                "positions": total_positions,
                "open_positions": open_positions,
                "strategy_count": strategy_specs.len(),
            }),
        )
        .await
        .wrap_err("failed to mark backtest completed")?;

    info!(
        run_id = %run_id,
        events_processed = result.events_processed,
        reports_generated = result.reports_generated,
        confirmed = result.confirmed_reports,
        failed = result.failed_reports,
        positions = total_positions,
        open_positions,
        "backtest finished"
    );

    Ok(())
}

fn build_strategy_specs(args: &Args) -> Result<Vec<BacktestStrategySpec>> {
    if let Some(suite) = args.strategy_suite.as_deref() {
        return match suite {
            "historical-maxhold" => Ok(historical_maxhold_suite_specs(args)),
            "mempool-history-exits" => Err(eyre::eyre!(
                "strategy suite mempool-history-exits was removed; historical backtests no longer replay mempool signals. Use --strategy-suite historical-maxhold"
            )),
            other => Err(eyre::eyre!("unsupported strategy suite: {other}")),
        };
    }

    Ok(vec![BacktestStrategySpec {
        strategy_name: args.strategy_name.clone(),
        strategy_impl: args.strategy_impl.clone(),
        stop_loss_ratio: args.stop_loss_ratio.clone(),
        take_profit_ratio: args.take_profit_ratio.clone(),
        max_hold_blocks: args.max_hold_blocks,
        exit_retry_interval_blocks: args.exit_retry_interval_blocks,
        max_exit_retries: args.max_exit_retries,
    }])
}

fn historical_maxhold_suite_specs(args: &Args) -> Vec<BacktestStrategySpec> {
    [10_u64, 20, 50]
        .into_iter()
        .map(|max_hold_blocks| BacktestStrategySpec {
            strategy_name: format!("snipe-all-maxhold{max_hold_blocks}"),
            strategy_impl: "snipe-all-v1".to_string(),
            stop_loss_ratio: args.stop_loss_ratio.clone(),
            take_profit_ratio: args.take_profit_ratio.clone(),
            max_hold_blocks: Some(max_hold_blocks),
            exit_retry_interval_blocks: args.exit_retry_interval_blocks,
            max_exit_retries: args.max_exit_retries,
        })
        .collect()
}

fn default_run_id() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    format!("alpha-backtest-{secs}-{}", std::process::id())
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

fn required_shared_config_value(config: &HashMap<String, String>, key: &str) -> Result<String> {
    config
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            eyre::eyre!(
                "{key} must be set in shared config file {}",
                shared_config_path().display()
            )
        })
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
