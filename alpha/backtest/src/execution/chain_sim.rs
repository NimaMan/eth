use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use eth_alpha_core::amount::Amount;
use eth_alpha_core::ids::{PoolAddress, StrategyName};
use eth_alpha_core::market::PoolSnapshot;
use eth_alpha_engine::{AlphaEngine, BlockCriticalRiskPolicy};
use eth_alpha_store::PostgresTradingStore;
use eth_strategies::{SnipeAllConfig, SnipeAllStrategy};
use eyre::{Result, WrapErr};
use rust_decimal::Decimal;
use tracing::info;

use crate::adapter::BacktestAdapter;
use crate::runner::run_backtest;
use crate::strategy_suites::BacktestStrategySpec;

#[derive(Clone, Debug)]
pub struct ChainSimBacktestConfig {
    pub replay_run_id: String,
    pub strategy_suite: Option<String>,
    pub execution_delay_blocks: u64,
}

pub async fn run_chain_sim_backtest(
    config: ChainSimBacktestConfig,
    store: PostgresTradingStore,
    events: Vec<eth_alpha_engine::EngineEvent>,
    run_id: String,
    strategy_specs: Vec<BacktestStrategySpec>,
    buy_amount: Amount,
    min_liquidity_eth: Decimal,
    min_liquidity_usd: Decimal,
    reth_datadir: &str,
) -> Result<()> {
    info!(reth_datadir = %reth_datadir, "initialising chain-sim execution adapter");
    let simulator = Arc::new(tx_simulator::TxSimulator::new(reth_datadir)?);
    let tx_processor = Arc::new(tx_processor::tx_processor::TxProcessor::new());
    let inner = eth_alpha_engine::execution::ChainSimExecutionAdapter::with_prefix(
        simulator,
        tx_processor,
        run_id.clone(),
    )?
    .with_execution_delay_blocks(config.execution_delay_blocks);
    let adapter = ChainSimBacktestAdapter::new(inner);

    run_backtest_with_adapter(
        config,
        store,
        events,
        adapter,
        run_id,
        strategy_specs,
        buy_amount,
        min_liquidity_eth,
        min_liquidity_usd,
    )
    .await
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
    config: ChainSimBacktestConfig,
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
                let lp_approval_gate_min_pct = spec
                    .lp_approval_gate_min_pct
                    .as_deref()
                    .and_then(|s| Decimal::from_str(s).ok());
                let min_sell_pool_denom_reserve = spec
                    .min_sell_pool_denom_reserve
                    .as_deref()
                    .and_then(|s| Decimal::from_str(s).ok())
                    .unwrap_or_else(|| SnipeAllConfig::default().min_sell_pool_denom_reserve);

                engine.add_strategy(Box::new(SnipeAllStrategy::new(SnipeAllConfig {
                    strategy_name: StrategyName(spec.strategy_name.clone()),
                    buy_amount: buy_amount.clone(),
                    sell_fraction: eth_alpha_core::amount::DecimalAmount::from(1),
                    min_denom_reserve: min_liquidity_eth,
                    min_stable_denom_reserve: min_liquidity_usd,
                    min_sell_pool_denom_reserve,
                    exit_on_liquidity_removal: spec.exit_on_liquidity_removal,
                    exit_on_tax: spec.exit_on_tax,
                    exit_on_lp_approval: spec.exit_on_lp_approval,
                    exit_on_critical_lp_approval_only: spec.exit_on_critical_lp_approval_only,
                    exit_on_scam: spec.exit_on_scam,
                    allowed_protocols: spec.allowed_protocols.clone(),
                    block_entry_on_lp_approval: spec.block_entry_on_lp_approval,
                    lp_approval_gate_min_pct,
                    defer_buy_confirm_block_lp_approval_to_max_hold: spec
                        .defer_buy_confirm_block_lp_approval_to_max_hold,
                    lp_approval_exit_defer_max_trading_enabled_age_blocks: spec
                        .lp_approval_exit_defer_max_trading_enabled_age_blocks,
                    stop_loss_ratio,
                    take_profit_ratio,
                    max_hold_blocks: spec.max_hold_blocks,
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
        replay_run_id = %config.replay_run_id,
        event_count = events.len(),
        strategy_count = strategy_specs.len(),
        strategy_suite = ?config.strategy_suite,
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
