use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use eth_alpha_backtest::{
    config::SimulationConfig,
    execution::SimulatedExecutionAdapter,
    runner::run_backtest,
};
use eth_alpha_core::amount::Amount;
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

    /// Additional slippage in basis points applied to sell-side fills.
    #[arg(long, default_value_t = 100)]
    slippage_bps: u32,

    /// Gas cost per simulated transaction (wei).
    #[arg(long, default_value_t = 150_000)]
    gas_cost_wei: u64,

    /// Random failure probability in basis points (0–10_000).
    #[arg(long, default_value_t = 0)]
    failure_rate_bps: u32,

    /// Allow trades on pools marked as scam.
    #[arg(long, default_value_t = false)]
    allow_scam: bool,

    /// Ignore liquidity thresholds.
    #[arg(long, default_value_t = false)]
    ignore_liquidity: bool,

    /// Use best-case fills instead of worst-case.
    #[arg(long, default_value_t = false)]
    best_case_fill: bool,

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

    let run_id = args.run_id.unwrap_or_else(default_run_id);

    let buy_amount = Amount {
        raw: alloy_primitives::U256::from_str_radix(&args.buy_amount_wei, 10)
            .wrap_err("invalid --buy-amount-wei")?,
        decimals: 18,
    };
    let min_liquidity_eth =
        Decimal::from_str(&args.min_liquidity_eth).wrap_err("invalid --min-liquidity-eth")?;
    let min_liquidity_usd =
        Decimal::from_str(&args.min_liquidity_usd).wrap_err("invalid --min-liquidity-usd")?;

    let simulation = SimulationConfig {
        slippage_bps: args.slippage_bps,
        gas_cost_wei: args.gas_cost_wei,
        failure_rate_bps: args.failure_rate_bps,
        min_denom_reserve: min_liquidity_eth,
        reject_scam: !args.allow_scam,
        reject_insufficient_liquidity: !args.ignore_liquidity,
        worst_case_fill: !args.best_case_fill,
    };

    let store = PostgresTradingStore::connect(&args.database_url, run_id.clone())
        .await
        .wrap_err("failed to connect to Postgres trading store")?;

    store
        .start_run(
            "backtest",
            serde_json::json!({
                "strategy_name": args.strategy_name,
                "replay_run_id": args.replay_run_id,
                "simulation": simulation,
                "min_liquidity_usd": min_liquidity_usd.to_string(),
            }),
        )
        .await
        .wrap_err("failed to start backtest run")?;

    let events = load_events_from_observations(store.pool(), &args.replay_run_id, args.skip_primed)
        .await
        .wrap_err_with(|| format!("failed to load observations for run {}", args.replay_run_id))?;

    if events.is_empty() {
        return Err(eyre::eyre!(
            "no valid events found for replay run {}",
            args.replay_run_id
        ));
    }

    let adapter = SimulatedExecutionAdapter::new(simulation.clone(), run_id.clone());

    let mut engine = AlphaEngine::new(
        BlockCriticalRiskPolicy,
        store.clone(),
        adapter.clone(),
    );

    match args.strategy_name.as_str() {
        "snipe-all-v1" => {
            engine.add_strategy(Box::new(SnipeAllStrategy::new(SnipeAllConfig {
                buy_amount: buy_amount.clone(),
                sell_fraction: eth_alpha_core::amount::DecimalAmount::from(1),
                min_denom_reserve: min_liquidity_eth,
                min_stable_denom_reserve: min_liquidity_usd,
                exit_on_liquidity_removal: args.exit_liquidity_removal,
                exit_on_tax: args.exit_tax,
                exit_on_lp_approval: args.exit_lp_approval,
                exit_on_scam: args.exit_scam,
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

/// Load historical events by replaying `strategy_observations` from an existing
/// live or paper run.
async fn load_events_from_observations(
    pool: &sqlx::PgPool,
    replay_run_id: &str,
    skip_primed: bool,
) -> Result<Vec<eth_alpha_engine::EngineEvent>> {
    let rows = sqlx::query(
        r#"
        SELECT event_source, event_key, block_number, payload
        FROM alpha_trading.strategy_observations
        WHERE run_id = $1
        ORDER BY block_number ASC NULLS LAST, first_seen_at ASC
        "#,
    )
    .bind(replay_run_id)
    .fetch_all(pool)
    .await
    .wrap_err("failed to query strategy_observations")?;

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

fn default_run_id() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    format!("alpha-backtest-{secs}-{}", std::process::id())
}
