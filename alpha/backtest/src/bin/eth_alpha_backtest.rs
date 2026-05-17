use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use alloy_primitives::Address;
use clap::Parser;
use eth_alpha_backtest::{adapter::BacktestAdapter, runner::run_backtest};
use eth_alpha_core::amount::Amount;
use eth_alpha_core::ids::{PoolAddress, StrategyName, TokenPoolId};
use eth_alpha_core::market::PoolSnapshot;
use eth_alpha_core::risk::{RiskEvent, RiskKind, RiskSeverity};
use eth_alpha_engine::wire::{
    decimal_from_f64, parse_address, parse_protocol, MempoolSignalWire, PoolWire,
};
use eth_alpha_engine::{AlphaEngine, BlockCriticalRiskPolicy};
use eth_alpha_store::PostgresTradingStore;
use eth_strategies::shared_rules::lp_approval_warning_exit;
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
    #[arg(long, default_value = "snipe-all")]
    strategy_name: String,

    /// Strategy implementation to instantiate.
    #[arg(long, default_value = "snipe-all")]
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

    /// Max hold active pool-update blocks: force sell after this many distinct
    /// pool-update blocks while the position is open.
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
    exit_on_liquidity_removal: bool,
    exit_on_tax: bool,
    exit_on_lp_approval: bool,
    exit_on_critical_lp_approval_only: bool,
    exit_on_scam: bool,
    allowed_protocols: Vec<String>,
    block_entry_on_lp_approval: bool,
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
            "exit_liquidity_removal": self.exit_on_liquidity_removal,
            "exit_tax": self.exit_on_tax,
            "exit_lp_approval": self.exit_on_lp_approval,
            "exit_lp_approval_critical_only": self.exit_on_critical_lp_approval_only,
            "exit_scam": self.exit_on_scam,
            "allowed_protocols": self.allowed_protocols,
            "block_entry_on_lp_approval": self.block_entry_on_lp_approval,
            "stop_loss_ratio": self.stop_loss_ratio,
            "take_profit_ratio": self.take_profit_ratio,
            "max_hold_blocks": self.max_hold_blocks,
            "exit_retry_interval_blocks": self.exit_retry_interval_blocks,
            "max_exit_retries": self.max_exit_retries,
        })
    }

    fn uses_signal_risk_events(&self) -> bool {
        self.exit_on_liquidity_removal
            || self.exit_on_lp_approval
            || self.exit_on_tax
            || self.exit_on_scam
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
    if !args.replay_run_id.starts_with("risk-atlas-") {
        validate_historical_signal_replay_names(&strategy_specs)?;
    }

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
    let include_historical_signal_risk_events = strategy_specs
        .iter()
        .any(BacktestStrategySpec::uses_signal_risk_events);

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
                "historical_signal_risk_events": include_historical_signal_risk_events,
            }),
        )
        .await
        .wrap_err("failed to start backtest run")?;

    let events = if args.replay_run_id.starts_with("risk-atlas-") {
        load_events_from_risk_atlas(store.pool(), &args.replay_run_id, args.from_block, args.to_block)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to load Risk Atlas observations for run {}",
                    args.replay_run_id
                )
            })?
    } else {
        load_events_from_observations(
            store.pool(),
            &args.replay_run_id,
            args.skip_primed,
            args.from_block,
            args.to_block,
            include_historical_signal_risk_events,
        )
        .await
        .wrap_err_with(|| format!("failed to load observations for run {}", args.replay_run_id))?
    };

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
    include_signal_risk_events: bool,
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
    let mut skipped_position_monitor = 0usize;
    let mut included_signal_risk_events = 0usize;

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
                if include_signal_risk_events {
                    let Some(observed_block) = observation_replay_block(row)? else {
                        skipped += 1;
                        continue;
                    };
                    let signal: MempoolSignalWire = match serde_json::from_value(
                        payload.get("signal").cloned().unwrap_or(Value::Null),
                    ) {
                        Ok(signal) => signal,
                        Err(error) => {
                            tracing::warn!(error = %error, "skipping malformed signal observation");
                            skipped += 1;
                            continue;
                        }
                    };
                    if let Some(event) = historical_signal_risk_event(signal, observed_block)? {
                        events.push(eth_alpha_engine::EngineEvent::Risk(event));
                        included_signal_risk_events += 1;
                    } else {
                        skipped_mempool += 1;
                    }
                    continue;
                }
                skipped_mempool += 1;
                continue;
            }
            "position_monitor" => {
                skipped_position_monitor += 1;
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
            skipped_position_monitor,
            included_signal_risk_events,
            total = rows.len(),
            "skipped historical observations"
        );
    }

    Ok(add_block_completed_events(events, from_block, to_block))
}

/// Load historical events from the token-lab Risk Atlas observation export.
///
/// This is the mined-chain replay path for token analytics features. It emits
/// pool update events from `risk_atlas_observations`, plus LP approval and
/// direct LP removal risk events derived from the observation event flags.
async fn load_events_from_risk_atlas(
    pool: &sqlx::PgPool,
    risk_atlas_run_id: &str,
    from_block: Option<u64>,
    to_block: Option<u64>,
) -> Result<Vec<eth_alpha_engine::EngineEvent>> {
    let mut query = String::from(
        r#"
        SELECT
            o.token_address,
            o.pool_address,
            o.denom_address,
            o.protocol,
            o.block_number,
            o.denom_reserve,
            o.token_reserve,
            o.can_buy,
            o.effective_can_buy,
            o.can_sell,
            o.effective_can_sell,
            o.direct_lp_removal_in_block,
            pe.quote_symbol,
            NULLIF(o.features->'token'->>'decimals', '')::INT AS token_decimals,
            NULLIF(o.features->'liquidity'->>'price_denom_per_token', '')::DOUBLE PRECISION AS price_denom_per_token,
            NULLIF(o.features->'liquidity'->>'initial_price_denom_per_token', '')::DOUBLE PRECISION AS initial_price_denom_per_token,
            NULLIF(o.features->'liquidity'->>'price_to_initial_ratio', '')::DOUBLE PRECISION AS price_ratio_to_initial,
            COALESCE(NULLIF(o.observation->'event_flags'->>'lp_approval_count_in_block', '')::INT, 0) AS lp_approval_count_in_block,
            NULLIF(o.features->'lp_control'->>'lp_total_supply', '')::DOUBLE PRECISION AS lp_total_supply,
            NULLIF(o.features->'lp_control'->>'lp_max_approval_amount_as_of', '')::DOUBLE PRECISION AS lp_max_approval_amount_as_of,
            NULLIF(o.features->'lp_control'->>'last_lp_approval_owner_is_creator', '')::BOOLEAN AS lp_approval_owner_is_creator
        FROM risk_atlas_observations o
        JOIN risk_atlas_pool_eligibility pe
          ON pe.run_id = o.run_id
         AND pe.token_address = o.token_address
         AND pe.pool_address = o.pool_address
        WHERE o.run_id = $1
          AND pe.eligible
          AND o.protocol = 'UNISWAP-V2'
        "#,
    );
    if from_block.is_some() {
        query.push_str(" AND o.block_number >= $2");
    }
    if to_block.is_some() {
        query.push_str(&format!(
            " AND o.block_number <= ${}",
            if from_block.is_some() { 3 } else { 2 }
        ));
    }
    query.push_str(" ORDER BY o.block_number ASC, o.token_address ASC, o.pool_address ASC");

    let mut q = sqlx::query(&query).bind(risk_atlas_run_id);
    if let Some(block) = from_block {
        q = q.bind(block as i64);
    }
    if let Some(block) = to_block {
        q = q.bind(block as i64);
    }

    let rows = q
        .fetch_all(pool)
        .await
        .wrap_err("failed to query risk_atlas_observations")?;

    let mut events = Vec::with_capacity(rows.len());
    let mut skipped = 0usize;
    let mut lp_approval_risks = 0usize;
    let mut direct_lp_removal_risks = 0usize;

    for row in rows {
        let block = u64::try_from(row.try_get::<i64, _>("block_number")?).unwrap_or_default();
        let token_address_text: String = row.try_get("token_address")?;
        let pool_identity: String = row.try_get("pool_address")?;
        let token_address = match parse_address(&token_address_text) {
            Ok(address) => address,
            Err(error) => {
                tracing::warn!(token_address = %token_address_text, error = %error, "skipping Risk Atlas row with invalid token address");
                skipped += 1;
                continue;
            }
        };
        let pool_address = TokenPoolId::new(token_address, &pool_identity);

        let lp_approval_count: i32 = row.try_get("lp_approval_count_in_block")?;
        if lp_approval_count > 0 {
            events.push(eth_alpha_engine::EngineEvent::Risk(RiskEvent {
                kind: RiskKind::LpApproval,
                severity: RiskSeverity::Warning,
                token_address,
                pool_address: Some(pool_address.clone()),
                pending_tx_hash: None,
                observed_block: Some(block),
                message: risk_atlas_lp_approval_message(&row, lp_approval_count)?,
            }));
            lp_approval_risks += 1;
        }

        if row.try_get::<bool, _>("direct_lp_removal_in_block")? {
            events.push(eth_alpha_engine::EngineEvent::Risk(RiskEvent {
                kind: RiskKind::LiquidityRemoval,
                severity: RiskSeverity::Critical,
                token_address,
                pool_address: Some(pool_address.clone()),
                pending_tx_hash: None,
                observed_block: Some(block),
                message: "risk atlas mined-chain direct LP liquidity removal".to_string(),
            }));
            direct_lp_removal_risks += 1;
        }

        let pool_snapshot = match risk_atlas_pool_snapshot(&row, token_address, pool_address, block)
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                tracing::warn!(error = %error, "skipping malformed Risk Atlas pool observation");
                skipped += 1;
                continue;
            }
        };
        events.push(eth_alpha_engine::EngineEvent::Market(
            eth_alpha_core::market::MarketEvent::PoolUpdated {
                block_number: block,
                pool: pool_snapshot,
            },
        ));
    }

    tracing::info!(
        risk_atlas_run_id,
        events = events.len(),
        skipped,
        lp_approval_risks,
        direct_lp_removal_risks,
        "loaded Risk Atlas historical events"
    );

    Ok(add_block_completed_events(events, from_block, to_block))
}

fn risk_atlas_pool_snapshot(
    row: &sqlx::postgres::PgRow,
    token_address: Address,
    pool_address: PoolAddress,
    block: u64,
) -> Result<PoolSnapshot> {
    let protocol: String = row.try_get("protocol")?;
    let denom_address_text: String = row.try_get("denom_address")?;
    let denom_address = parse_address(&denom_address_text)?;
    let denom_reserve = row.try_get::<Option<f64>, _>("denom_reserve")?.unwrap_or(0.0);
    let token_reserve = row.try_get::<Option<f64>, _>("token_reserve")?.unwrap_or(0.0);
    let can_buy = row
        .try_get::<Option<bool>, _>("effective_can_buy")?
        .unwrap_or(row.try_get::<bool, _>("can_buy")?);
    let can_sell = row
        .try_get::<Option<bool>, _>("effective_can_sell")?
        .unwrap_or(row.try_get::<bool, _>("can_sell")?);
    let token_decimals = row
        .try_get::<Option<i32>, _>("token_decimals")?
        .and_then(|decimals| u8::try_from(decimals).ok());

    Ok(PoolSnapshot {
        address: pool_address,
        token_address,
        protocol: parse_protocol(&protocol),
        denom_address: Some(denom_address),
        denom_symbol: row.try_get::<Option<String>, _>("quote_symbol")?,
        denom_reserve: decimal_from_f64(denom_reserve),
        token_reserve: decimal_from_f64(token_reserve),
        price_denom_per_token: row
            .try_get::<Option<f64>, _>("price_denom_per_token")?
            .map(decimal_from_f64),
        initial_price_denom_per_token: row
            .try_get::<Option<f64>, _>("initial_price_denom_per_token")?
            .map(decimal_from_f64),
        price_ratio_to_initial: row
            .try_get::<Option<f64>, _>("price_ratio_to_initial")?
            .map(decimal_from_f64),
        token_decimals,
        fee_tier: None,
        uniswap_v4: None,
        latest_block: block,
        can_buy,
        can_sell,
        is_scam: false,
    })
}

fn risk_atlas_lp_approval_message(
    row: &sqlx::postgres::PgRow,
    lp_approval_count: i32,
) -> Result<String> {
    let lp_total_supply = row.try_get::<Option<f64>, _>("lp_total_supply")?;
    let max_approval = row.try_get::<Option<f64>, _>("lp_max_approval_amount_as_of")?;
    let owner_is_creator = row
        .try_get::<Option<bool>, _>("lp_approval_owner_is_creator")?
        .unwrap_or(false);
    let approval_pct = lp_total_supply
        .zip(max_approval)
        .and_then(|(supply, approval)| {
            if supply > 0.0 {
                Some((approval / supply * 100.0).min(100.0))
            } else {
                None
            }
        });
    Ok(format!(
        "risk atlas mined-chain LP approval: count={lp_approval_count}, generic_approved_pct={}, owner_is_creator={owner_is_creator}",
        approval_pct
            .map(|pct| format!("{pct:.2}%"))
            .unwrap_or_else(|| "unknown".to_string())
    ))
}

fn observation_replay_block(row: &sqlx::postgres::PgRow) -> Result<Option<u64>> {
    Ok(row
        .try_get::<Option<i64>, _>("replay_block")?
        .and_then(|block| u64::try_from(block).ok()))
}

fn historical_signal_risk_event(
    signal: MempoolSignalWire,
    observed_block: u64,
) -> Result<Option<eth_alpha_core::risk::RiskEvent>> {
    if !matches!(
        signal.signal_type.as_str(),
        "lp_approval" | "lp_position_approval" | "liquidity_removal"
    ) {
        return Ok(None);
    }

    let Some(mut event) = signal.to_risk_event()? else {
        return Ok(None);
    };
    event.observed_block = Some(observed_block);
    event.message = format!("historical confirmed signal: {}", event.message);
    Ok(Some(event))
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
            "snipe-all" => {
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
                    exit_on_liquidity_removal: spec.exit_on_liquidity_removal,
                    exit_on_tax: spec.exit_on_tax,
                    exit_on_lp_approval: spec.exit_on_lp_approval,
                    exit_on_critical_lp_approval_only: spec.exit_on_critical_lp_approval_only,
                    exit_on_scam: spec.exit_on_scam,
                    allowed_protocols: spec.allowed_protocols.clone(),
                    block_entry_on_lp_approval: spec.block_entry_on_lp_approval,
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
            "historical-pool-update-hold" => Ok(historical_pool_update_hold_suite_specs(args)),
            "risk-atlas-edge-v1" => Ok(risk_atlas_edge_suite_v1_specs(args)),
            "risk-atlas-edge-v2" => Ok(risk_atlas_edge_suite_v2_specs(args)),
            lp_approval_warning_exit::MEMPOOL_AWARE_HISTORICAL_SUITE_NAME
            | lp_approval_warning_exit::MEMPOOL_AWARE_HISTORICAL_STRATEGY_NAME => {
                Ok(vec![historical_mempool_aware_lp_approval_warning_exit_spec(args)])
            }
            lp_approval_warning_exit::SUITE_NAME | lp_approval_warning_exit::STRATEGY_NAME => {
                Err(eyre::eyre!(
                    "historical replay of stored mempool_signal rows must use a mempool-aware name. Use --strategy-suite {}",
                    lp_approval_warning_exit::MEMPOOL_AWARE_HISTORICAL_SUITE_NAME
                ))
            }
            "mempool-history-exits" => Err(eyre::eyre!(
                "strategy suite mempool-history-exits was removed; historical backtests no longer replay mempool signals. Use --strategy-suite historical-pool-update-hold"
            )),
            other => Err(eyre::eyre!("unsupported strategy suite: {other}")),
        };
    }

    Ok(vec![BacktestStrategySpec {
        strategy_name: args.strategy_name.clone(),
        strategy_impl: args.strategy_impl.clone(),
        exit_on_liquidity_removal: false,
        exit_on_tax: false,
        exit_on_lp_approval: false,
        exit_on_critical_lp_approval_only: false,
        exit_on_scam: false,
        allowed_protocols: Vec::new(),
        block_entry_on_lp_approval: false,
        stop_loss_ratio: args.stop_loss_ratio.clone(),
        take_profit_ratio: args.take_profit_ratio.clone(),
        max_hold_blocks: args.max_hold_blocks,
        exit_retry_interval_blocks: args.exit_retry_interval_blocks,
        max_exit_retries: args.max_exit_retries,
    }])
}

fn historical_pool_update_hold_suite_specs(args: &Args) -> Vec<BacktestStrategySpec> {
    [1_u64, 2, 3, 5, 10]
        .into_iter()
        .map(|max_hold_blocks| BacktestStrategySpec {
            strategy_name: format!("snipe-all-hold{max_hold_blocks}-pool-updates"),
            strategy_impl: "snipe-all".to_string(),
            exit_on_liquidity_removal: false,
            exit_on_tax: false,
            exit_on_lp_approval: false,
            exit_on_critical_lp_approval_only: false,
            exit_on_scam: false,
            allowed_protocols: Vec::new(),
            block_entry_on_lp_approval: false,
            stop_loss_ratio: args.stop_loss_ratio.clone(),
            take_profit_ratio: args.take_profit_ratio.clone(),
            max_hold_blocks: Some(max_hold_blocks),
            exit_retry_interval_blocks: args.exit_retry_interval_blocks,
            max_exit_retries: args.max_exit_retries,
        })
        .collect()
}

fn risk_atlas_edge_suite_v1_specs(args: &Args) -> Vec<BacktestStrategySpec> {
    vec![
        risk_atlas_spec(
            "snipe-all-risk-atlas-lp-immediate-exit-v1",
            true,
            false,
            None,
            args,
        ),
        risk_atlas_spec(
            "snipe-all-risk-atlas-lp-launch-gate-v1",
            true,
            true,
            None,
            args,
        ),
        risk_atlas_spec(
            "snipe-all-risk-atlas-active-horizon-hold10-v1",
            true,
            true,
            Some(10),
            args,
        ),
        risk_atlas_spec(
            "snipe-all-risk-atlas-v2-backdoor-fast-hold5-v1",
            false,
            true,
            Some(5),
            args,
        ),
        risk_atlas_spec(
            "snipe-all-risk-atlas-protocol-guard-hold50-v1",
            true,
            true,
            Some(50),
            args,
        ),
    ]
}

fn risk_atlas_edge_suite_v2_specs(args: &Args) -> Vec<BacktestStrategySpec> {
    [5_u64, 8, 10, 12, 15]
        .into_iter()
        .map(|max_hold_blocks| {
            risk_atlas_spec(
                &format!("snipe-all-risk-atlas-lp-gate-hold{max_hold_blocks}-v2"),
                true,
                true,
                Some(max_hold_blocks),
                args,
            )
        })
        .collect()
}

fn risk_atlas_spec(
    strategy_name: &str,
    exit_on_lp_approval: bool,
    block_entry_on_lp_approval: bool,
    max_hold_blocks: Option<u64>,
    args: &Args,
) -> BacktestStrategySpec {
    BacktestStrategySpec {
        strategy_name: strategy_name.to_string(),
        strategy_impl: "snipe-all".to_string(),
        exit_on_liquidity_removal: true,
        exit_on_tax: false,
        exit_on_lp_approval,
        exit_on_critical_lp_approval_only: false,
        exit_on_scam: false,
        allowed_protocols: vec!["UNISWAP-V2".to_string()],
        block_entry_on_lp_approval,
        stop_loss_ratio: args.stop_loss_ratio.clone(),
        take_profit_ratio: args.take_profit_ratio.clone(),
        max_hold_blocks,
        exit_retry_interval_blocks: args.exit_retry_interval_blocks,
        max_exit_retries: args.max_exit_retries,
    }
}

fn historical_mempool_aware_lp_approval_warning_exit_spec(args: &Args) -> BacktestStrategySpec {
    BacktestStrategySpec {
        strategy_name: lp_approval_warning_exit::MEMPOOL_AWARE_HISTORICAL_STRATEGY_NAME.to_string(),
        strategy_impl: "snipe-all".to_string(),
        exit_on_liquidity_removal: lp_approval_warning_exit::EXIT_LIQUIDITY_REMOVAL,
        exit_on_tax: lp_approval_warning_exit::EXIT_TAX,
        exit_on_lp_approval: lp_approval_warning_exit::EXIT_LP_APPROVAL,
        exit_on_critical_lp_approval_only: lp_approval_warning_exit::EXIT_LP_APPROVAL_CRITICAL_ONLY,
        exit_on_scam: lp_approval_warning_exit::EXIT_SCAM,
        allowed_protocols: Vec::new(),
        block_entry_on_lp_approval: false,
        stop_loss_ratio: args.stop_loss_ratio.clone(),
        take_profit_ratio: args.take_profit_ratio.clone(),
        max_hold_blocks: None,
        exit_retry_interval_blocks: args.exit_retry_interval_blocks,
        max_exit_retries: args.max_exit_retries,
    }
}

fn validate_historical_signal_replay_names(specs: &[BacktestStrategySpec]) -> Result<()> {
    for spec in specs {
        if spec.uses_signal_risk_events() && !spec.strategy_name.contains("mempool-aware") {
            return Err(eyre::eyre!(
                "historical strategy {} replays stored mempool_signal risk events but is not named mempool-aware",
                spec.strategy_name
            ));
        }
    }
    Ok(())
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
