use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use eth_alpha_backtest::{
    adapter::BacktestAdapter,
    runner::run_backtest,
};
use eth_alpha_core::amount::Amount;
use eth_alpha_core::ids::PoolAddress;
use eth_alpha_core::market::PoolSnapshot;
use eth_alpha_engine::{AlphaEngine, BlockCriticalRiskPolicy};
use eth_alpha_engine::wire::{MempoolSignalWire, PoolWire};
use eth_alpha_store::PostgresTradingStore;
use eth_strategies::{SnipeAllConfig, SnipeAllStrategy};
use eyre::{Result, WrapErr};
use rust_decimal::Decimal;
use serde_json::Value;
use sqlx::Row;
use tracing::info;

#[derive(Debug, Parser)]
struct Args {
    /// PostgreSQL database URL.
    #[arg(long, env = "ALPHA_DATABASE_URL")]
    database_url: String,

    /// Unique run identifier for this backtest.
    #[arg(long, env = "ALPHA_BACKTEST_RUN_ID")]
    run_id: Option<String>,

    /// Strategy to run.
    #[arg(long, default_value = "snipe-all-v1")]
    strategy_name: String,

    /// Existing live or paper run_id to replay from `strategy_observations`.
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

    /// Enable liquidity-removal exits (default: disabled for quantification).
    #[arg(long, default_value_t = false)]
    exit_liquidity_removal: bool,

    /// Enable tax/honeypot exits (default: disabled for quantification).
    #[arg(long, default_value_t = false)]
    exit_tax: bool,

    /// Enable LP-approval exits (default: disabled for quantification).
    #[arg(long, default_value_t = false)]
    exit_lp_approval: bool,

    /// Enable scam/critical-risk exits (default: disabled for quantification).
    #[arg(long, default_value_t = false)]
    exit_scam: bool,

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

    /// Path to the Reth database directory.
    #[arg(long, default_value = "/home/nima/storage/samsung8tb/ethereum/reth")]
    reth_datadir: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();

    let run_id = args.run_id.clone().unwrap_or_else(default_run_id);

    let buy_amount = Amount {
        raw: alloy_primitives::U256::from_str_radix(&args.buy_amount_wei, 10)
            .wrap_err("invalid --buy-amount-wei")?,
        decimals: 18,
    };
    let min_liquidity_eth =
        Decimal::from_str(&args.min_liquidity_eth).wrap_err("invalid --min-liquidity-eth")?;
    let min_liquidity_usd =
        Decimal::from_str(&args.min_liquidity_usd).wrap_err("invalid --min-liquidity-usd")?;

    let store = PostgresTradingStore::connect(&args.database_url, run_id.clone())
        .await
        .wrap_err("failed to connect to Postgres trading store")?;

    store
        .start_run(
            "backtest",
            serde_json::json!({
                "strategy_name": args.strategy_name,
                "replay_run_id": args.replay_run_id,
                "min_liquidity_usd": min_liquidity_usd.to_string(),
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

    info!(reth_datadir = %args.reth_datadir, "initialising EVM-backed simulation adapter");
    let simulator = Arc::new(tx_simulator::TxSimulator::new(&args.reth_datadir)?);
    let tx_processor = Arc::new(tx_processor::tx_processor::TxProcessor::new());
    let inner = eth_alpha_engine::execution::SimulatedExecutionAdapter::with_prefix(
        simulator,
        tx_processor,
        run_id.clone(),
    )?;
    let adapter = EvmBacktestAdapter::new(inner);

    run_backtest_with_adapter(
        args,
        store,
        events,
        adapter,
        run_id,
        buy_amount,
        min_liquidity_eth,
        min_liquidity_usd,
    )
    .await?;

    Ok(())
}

/// Load historical events by replaying `strategy_observations` from an existing
/// live or paper run.
async fn load_events_from_observations(
    pool: &sqlx::PgPool,
    replay_run_id: &str,
    skip_primed: bool,
    from_block: Option<u64>,
    to_block: Option<u64>,
) -> Result<Vec<eth_alpha_engine::EngineEvent>> {
    let mut query = String::from(
        "SELECT event_source, event_key, block_number, payload
         FROM alpha_trading.strategy_observations
         WHERE run_id = $1"
    );
    if from_block.is_some() {
        query.push_str(" AND block_number >= $2");
    }
    if to_block.is_some() {
        query.push_str(&format!(" AND block_number <= ${}",
            if from_block.is_some() { 3 } else { 2 }));
    }
    query.push_str(" ORDER BY block_number ASC NULLS LAST, first_seen_at ASC");

    let mut q = sqlx::query(&query).bind(replay_run_id);
    if let Some(b) = from_block {
        q = q.bind(b as i64);
    }
    if let Some(b) = to_block {
        q = q.bind(b as i64);
    }
    let rows = q.fetch_all(pool).await.wrap_err("failed to query strategy_observations")?;

    let mut events = Vec::with_capacity(rows.len());
    let mut skipped = 0usize;

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
                let signal_wire: MempoolSignalWire = match serde_json::from_value(
                    payload.get("signal").cloned().unwrap_or(Value::Null),
                ) {
                    Ok(w) => w,
                    Err(error) => {
                        tracing::warn!(error = %error, "skipping malformed signal observation");
                        skipped += 1;
                        continue;
                    }
                };
                match signal_wire.to_risk_event() {
                    Ok(Some(risk_event)) => {
                        events.push(eth_alpha_engine::EngineEvent::Risk(risk_event));
                    }
                    Ok(None) => {}
                    Err(error) => {
                        tracing::warn!(error = %error, "skipping signal with missing fields");
                        skipped += 1;
                    }
                }
            }
            other => {
                tracing::warn!(event_source = %other, "skipping unknown observation source");
                skipped += 1;
            }
        }
    }

    if skipped > 0 {
        tracing::info!(skipped, total = rows.len(), "skipped malformed observations");
    }

    Ok(events)
}

#[derive(Clone)]
struct EvmBacktestAdapter(eth_alpha_engine::execution::SimulatedExecutionAdapter);

impl EvmBacktestAdapter {
    fn new(inner: eth_alpha_engine::execution::SimulatedExecutionAdapter) -> Self {
        Self(inner)
    }
}

#[async_trait::async_trait]
impl eth_alpha_engine::EngineExecutionAdapter for EvmBacktestAdapter {
    async fn execute(&self, intent: eth_alpha_core::order::OrderIntent) -> eth_alpha_core::error::Result<eth_alpha_core::execution::ExecutionReport> {
        self.0.execute(intent).await
    }
}

impl BacktestAdapter for EvmBacktestAdapter {
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
    buy_amount: Amount,
    min_liquidity_eth: Decimal,
    min_liquidity_usd: Decimal,
) -> Result<()>
where
    A: eth_alpha_engine::EngineExecutionAdapter + BacktestAdapter + Clone,
{
    let mut engine = AlphaEngine::new(
        BlockCriticalRiskPolicy,
        store.clone(),
        adapter.clone(),
    );

    match args.strategy_name.as_str() {
        "snipe-all-v1" => {
            let stop_loss_ratio = args
                .stop_loss_ratio
                .as_deref()
                .and_then(|s| Decimal::from_str(s).ok());
            let take_profit_ratio = args
                .take_profit_ratio
                .as_deref()
                .and_then(|s| Decimal::from_str(s).ok());

            engine.add_strategy(Box::new(SnipeAllStrategy::new(SnipeAllConfig {
                buy_amount,
                sell_fraction: eth_alpha_core::amount::DecimalAmount::from(1),
                min_denom_reserve: min_liquidity_eth,
                min_stable_denom_reserve: min_liquidity_usd,
                exit_on_liquidity_removal: args.exit_liquidity_removal,
                exit_on_tax: args.exit_tax,
                exit_on_lp_approval: args.exit_lp_approval,
                exit_on_scam: args.exit_scam,
                stop_loss_ratio,
                take_profit_ratio,
                max_hold_blocks: args.max_hold_blocks,
                ..SnipeAllConfig::default()
            })));
        }
        other => {
            return Err(eyre::eyre!("unsupported strategy: {other}"));
        }
    }

    info!(
        run_id = %run_id,
        replay_run_id = %args.replay_run_id,
        event_count = events.len(),
        strategy = %args.strategy_name,
        "starting backtest"
    );

    let result = run_backtest(&mut engine, &adapter, events).await?;

    store
        .mark_stopped(
            "completed",
            serde_json::json!({
                "events_processed": result.events_processed,
                "reports_generated": result.reports_generated,
                "confirmed_reports": result.confirmed_reports,
                "failed_reports": result.failed_reports,
                "positions": engine.portfolio().active_position_count(),
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
        open_positions = engine.portfolio().active_position_count(),
        "backtest finished"
    );

    Ok(())
}

fn default_run_id() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    format!("alpha-backtest-{secs}-{}", std::process::id())
}
