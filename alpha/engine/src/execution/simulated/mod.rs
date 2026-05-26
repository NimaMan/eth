//! Chain-state EVM execution adapters.
//!
//! Two variants are provided:
//!
//! | Adapter | Use Case | Block Source |
//! |---------|----------|-------------|
//! | `ChainSimExecutionAdapter` | Historical backtest | Manually-set `current_block` |
//! | `LiveChainSimExecutionAdapter` | Live no-capital trading | Observed event block, gated by live chain state |
//!
//! Both run the actual swap calldata through the EVM so that token taxes, max
//! transaction limits, and other contract-level behaviour are captured from
//! chain state. No snapshot-price or perfect-fill fallback is allowed here.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use eth_alpha_core::{
    error::Result,
    execution::ExecutionReport,
    ids::{OrderId, PoolAddress},
    market::PoolSnapshot,
    order::{OrderIntent, OrderSide},
    portfolio::PortfolioState,
    position::Position,
};
use tokio::time::sleep;
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::{LiveTxSimulator, TxSimulator};

use crate::{EngineExecutionAdapter, PositionValueSimulation};

mod pool_context;
mod reports;
mod swaps;

use reports::{
    failed_report, failed_report_at, position_value_from_report, sell_intent_for_position,
    submitted_live_chain_sim_report, unique_order_prefix, with_live_chain_sim_evidence,
};
use swaps::{
    simulate_buy_at_block, simulate_live_buy_at_block, simulate_live_sell_at_block,
    simulate_sell_at_block,
};

const LIVE_EXECUTION_DELAY_BLOCKS: u64 = 1;
const LIVE_STATE_WAIT_TIMEOUT: Duration = Duration::from_secs(45);
const LIVE_STATE_WAIT_INTERVAL: Duration = Duration::from_millis(500);

// ---------------------------------------------------------------------------
// Historical backtest adapter
// ---------------------------------------------------------------------------

/// Historical chain simulation adapter.
///
/// Runs swaps against a specific block number that is set externally by the
/// backtest runner.  Use this when replaying historical observations.
#[derive(Clone)]
pub struct ChainSimExecutionAdapter {
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    order_prefix: Arc<str>,
    next_order_id: Arc<AtomicU64>,
    pools: Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>>,
    portfolio: Arc<Mutex<PortfolioState>>,
    current_block: Arc<AtomicU64>,
    execution_delay_blocks: u64,
}

impl ChainSimExecutionAdapter {
    pub fn new(simulator: Arc<TxSimulator>, tx_processor: Arc<TxProcessor>) -> Result<Self> {
        Ok(Self {
            simulator,
            tx_processor,
            order_prefix: Arc::<str>::from(unique_order_prefix()),
            next_order_id: Arc::new(AtomicU64::new(0)),
            pools: Arc::new(Mutex::new(HashMap::new())),
            portfolio: Arc::new(Mutex::new(PortfolioState::default())),
            current_block: Arc::new(AtomicU64::new(0)),
            execution_delay_blocks: 0,
        })
    }

    pub fn with_prefix(
        simulator: Arc<TxSimulator>,
        tx_processor: Arc<TxProcessor>,
        prefix: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            simulator,
            tx_processor,
            order_prefix: Arc::<str>::from(prefix.into()),
            next_order_id: Arc::new(AtomicU64::new(0)),
            pools: Arc::new(Mutex::new(HashMap::new())),
            portfolio: Arc::new(Mutex::new(PortfolioState::default())),
            current_block: Arc::new(AtomicU64::new(0)),
            execution_delay_blocks: 0,
        })
    }

    /// Delay final simulated fills by this many blocks after the observed
    /// decision block. A delay of 1 models observing block N, submitting at N,
    /// and filling against post-block N+1 state.
    pub fn with_execution_delay_blocks(mut self, delay_blocks: u64) -> Self {
        self.execution_delay_blocks = delay_blocks;
        self
    }

    /// Shared handle to the live pool map.
    pub fn pools(&self) -> Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>> {
        self.pools.clone()
    }

    /// Shared handle to the portfolio state.
    pub fn portfolio(&self) -> Arc<Mutex<PortfolioState>> {
        self.portfolio.clone()
    }

    /// Shared handle to the current block number.
    pub fn current_block(&self) -> Arc<AtomicU64> {
        self.current_block.clone()
    }
}

#[async_trait]
impl EngineExecutionAdapter for ChainSimExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-{order_seq}", self.order_prefix));

        let observed_block = self.current_block.load(Ordering::Relaxed);
        let pool = {
            let pools = self.pools.lock().expect("pool lock");
            let Some(pool) = pools.get(&intent.pool_address).cloned() else {
                return Ok(if observed_block == 0 {
                    failed_report(order_id, "pool not in simulation state")
                } else {
                    failed_report_at(order_id, "pool not in simulation state", observed_block)
                });
            };
            pool
        };

        if observed_block == 0 {
            return Ok(failed_report(order_id, "current block not set"));
        }
        let Some(execution_block) = observed_block.checked_add(self.execution_delay_blocks) else {
            return Ok(failed_report_at(
                order_id,
                "execution block overflow",
                observed_block,
            ));
        };

        match intent.side {
            OrderSide::Buy => {
                simulate_buy_at_block(
                    &self.simulator,
                    &self.tx_processor,
                    order_id,
                    intent,
                    &pool,
                    execution_block,
                )
                .await
            }
            OrderSide::Sell => {
                simulate_sell_at_block(
                    &self.simulator,
                    &self.tx_processor,
                    order_id,
                    intent,
                    &pool,
                    execution_block,
                    &self.portfolio,
                    true,
                )
                .await
            }
        }
    }

    async fn simulate_position_value(
        &self,
        position: &Position,
        pool: &PoolSnapshot,
    ) -> Result<Option<PositionValueSimulation>> {
        let Some(intent) = sell_intent_for_position(position) else {
            return Ok(None);
        };
        let block = self.current_block.load(Ordering::Relaxed);
        if block == 0 {
            return Ok(None);
        }
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-value-{order_seq}", self.order_prefix));
        let report = simulate_sell_at_block(
            &self.simulator,
            &self.tx_processor,
            order_id,
            intent,
            pool,
            block,
            &self.portfolio,
            false,
        )
        .await?;
        Ok(position_value_from_report(report, block))
    }
}

// ---------------------------------------------------------------------------
// Live trading adapter
// ---------------------------------------------------------------------------

/// Live chain simulation adapter.
///
/// Uses only the live block session published from the chain-server state
/// frame. It does not select a "latest" block from local Reth historical
/// context.
#[derive(Clone)]
pub struct LiveChainSimExecutionAdapter {
    live_sim: LiveTxSimulator,
    tx_processor: Arc<TxProcessor>,
    order_prefix: Arc<str>,
    next_order_id: Arc<AtomicU64>,
    current_block: Arc<AtomicU64>,
    pools: Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>>,
    portfolio: Arc<Mutex<PortfolioState>>,
    execution_delay_blocks: u64,
}

impl LiveChainSimExecutionAdapter {
    pub fn new(live_sim: LiveTxSimulator, tx_processor: Arc<TxProcessor>) -> Result<Self> {
        Ok(Self {
            live_sim,
            tx_processor,
            order_prefix: Arc::<str>::from(unique_order_prefix()),
            next_order_id: Arc::new(AtomicU64::new(0)),
            current_block: Arc::new(AtomicU64::new(0)),
            pools: Arc::new(Mutex::new(HashMap::new())),
            portfolio: Arc::new(Mutex::new(PortfolioState::default())),
            execution_delay_blocks: LIVE_EXECUTION_DELAY_BLOCKS,
        })
    }

    pub fn with_prefix(
        live_sim: LiveTxSimulator,
        tx_processor: Arc<TxProcessor>,
        prefix: impl Into<String>,
    ) -> Result<Self> {
        Self::with_prefix_and_next_order_sequence(live_sim, tx_processor, prefix, 0)
    }

    pub fn with_prefix_and_next_order_sequence(
        live_sim: LiveTxSimulator,
        tx_processor: Arc<TxProcessor>,
        prefix: impl Into<String>,
        next_order_sequence: u64,
    ) -> Result<Self> {
        Ok(Self {
            live_sim,
            tx_processor,
            order_prefix: Arc::<str>::from(prefix.into()),
            next_order_id: Arc::new(AtomicU64::new(next_order_sequence)),
            current_block: Arc::new(AtomicU64::new(0)),
            pools: Arc::new(Mutex::new(HashMap::new())),
            portfolio: Arc::new(Mutex::new(PortfolioState::default())),
            execution_delay_blocks: LIVE_EXECUTION_DELAY_BLOCKS,
        })
    }

    pub fn current_block(&self) -> Arc<AtomicU64> {
        self.current_block.clone()
    }

    pub fn live_simulator(&self) -> LiveTxSimulator {
        self.live_sim.clone()
    }

    pub fn with_execution_delay_blocks(mut self, delay_blocks: u64) -> Self {
        self.execution_delay_blocks = delay_blocks;
        self
    }

    pub fn pools(&self) -> Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>> {
        self.pools.clone()
    }

    pub fn portfolio(&self) -> Arc<Mutex<PortfolioState>> {
        self.portfolio.clone()
    }

    /// Expose diagnostics about which state source is being used.
    pub async fn state_status(&self) -> eyre::Result<tx_simulator::LiveStateStatus> {
        self.live_sim.latest_state_status().await
    }

    async fn wait_for_execution_block(
        &self,
        target_block: u64,
    ) -> std::result::Result<u64, String> {
        let started = Instant::now();
        loop {
            match self.live_sim.latest_state_block_number().await {
                Ok(selected_block) if selected_block == target_block => return Ok(target_block),
                Ok(selected_block) if selected_block > target_block => {
                    if self
                        .live_sim
                        .has_state_at(target_block)
                        .await
                        .unwrap_or(false)
                    {
                        return Ok(target_block);
                    }
                    return Err(format!(
                        "live chain-sim missed required execution block {target_block}; latest live state is {selected_block}"
                    ));
                }
                Ok(selected_block) if started.elapsed() >= LIVE_STATE_WAIT_TIMEOUT => {
                    return Err(format!(
                        "live chain-sim state stale: selected block {selected_block} below required execution block {target_block}"
                    ));
                }
                Ok(_) => {}
                Err(error) if started.elapsed() >= LIVE_STATE_WAIT_TIMEOUT => {
                    return Err(format!(
                        "live chain-sim state unavailable before required execution block {target_block}: {error}"
                    ));
                }
                Err(_) => {}
            }
            sleep(LIVE_STATE_WAIT_INTERVAL).await;
        }
    }

    pub async fn simulate_submitted_order(
        &self,
        order_id: OrderId,
        intent: OrderIntent,
        submitted_block: u64,
        execution_block: u64,
    ) -> Result<ExecutionReport> {
        let pool = {
            let pools = self.pools.lock().expect("pool lock");
            let Some(pool) = pools.get(&intent.pool_address).cloned() else {
                return Ok(with_live_chain_sim_evidence(
                    failed_report_at(order_id, "pool not in simulation state", execution_block),
                    submitted_block,
                    execution_block,
                    None,
                ));
            };
            pool
        };

        let report = match intent.side {
            OrderSide::Buy => {
                simulate_live_buy_at_block(
                    &self.live_sim,
                    &self.tx_processor,
                    order_id,
                    intent,
                    &pool,
                    execution_block,
                )
                .await
            }
            OrderSide::Sell => {
                simulate_live_sell_at_block(
                    &self.live_sim,
                    &self.tx_processor,
                    order_id,
                    intent,
                    &pool,
                    execution_block,
                    &self.portfolio,
                    true,
                )
                .await
            }
        }?;
        Ok(with_live_chain_sim_evidence(
            report,
            submitted_block,
            execution_block,
            Some(execution_block),
        ))
    }
}

#[async_trait]
impl EngineExecutionAdapter for LiveChainSimExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-{order_seq}", self.order_prefix));

        let pool = {
            let pools = self.pools.lock().expect("pool lock");
            let Some(pool) = pools.get(&intent.pool_address).cloned() else {
                return Ok(failed_report(order_id, "pool not in simulation state"));
            };
            pool
        };

        let observed_block = self.current_block.load(Ordering::Relaxed);
        let observed_block = if observed_block > 0 {
            observed_block
        } else {
            pool.latest_block
        };
        let Some(target_block) = observed_block.checked_add(self.execution_delay_blocks) else {
            return Ok(failed_report_at(
                order_id,
                "live chain-sim execution block overflow",
                observed_block,
            ));
        };
        Ok(submitted_live_chain_sim_report(
            order_id,
            observed_block,
            target_block,
        ))
    }

    async fn simulate_position_value(
        &self,
        position: &Position,
        pool: &PoolSnapshot,
    ) -> Result<Option<PositionValueSimulation>> {
        let Some(intent) = sell_intent_for_position(position) else {
            return Ok(None);
        };
        let observed_block = self.current_block.load(Ordering::Relaxed);
        let observed_block = if observed_block > 0 {
            observed_block
        } else {
            pool.latest_block
        };
        let block = match self.wait_for_execution_block(observed_block).await {
            Ok(block) => block,
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    observed_block,
                    "chain-sim position valuation skipped because required state block is unavailable"
                );
                return Ok(None);
            }
        };
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-value-{order_seq}", self.order_prefix));
        let report = simulate_live_sell_at_block(
            &self.live_sim,
            &self.tx_processor,
            order_id,
            intent,
            pool,
            block,
            &self.portfolio,
            false,
        )
        .await?;
        Ok(position_value_from_report(report, block))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use eth_alpha_core::{amount::Amount, execution::ExecutionStatus};

    use super::*;

    #[test]
    fn failed_position_valuation_returns_zero_value_snapshot_input() {
        let report = ExecutionReport {
            order_id: OrderId("valuation-1".to_string()),
            status: ExecutionStatus::Failed,
            tx_hash: None,
            block_number: None,
            filled_amount: None,
            token_amount: None,
            gas_used: Some(123),
            gas_cost: None,
            mined_evidence: None,
            error: Some("unable to inject synthetic ERC20 balance".to_string()),
        };

        let value = position_value_from_report(report, 42).expect("zero valuation");
        assert_eq!(value.block_number, 42);
        assert_eq!(value.current_value, Amount::zero(18));
        assert_eq!(value.gas_used, Some(123));
        assert_eq!(
            value.error.as_deref(),
            Some("unable to inject synthetic ERC20 balance")
        );
    }
}
