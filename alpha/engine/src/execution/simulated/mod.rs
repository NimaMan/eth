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

use alloy_primitives::B256;
use async_trait::async_trait;
use eth_alpha_core::{
    error::{AlphaCoreError, Result},
    execution::ExecutionReport,
    ids::{OrderId, PoolAddress},
    market::PoolSnapshot,
    order::{OrderIntent, OrderSide},
    portfolio::PortfolioState,
    position::Position,
};
use serde::{Deserialize, Serialize};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

use crate::{EngineExecutionAdapter, PositionValueSimulation};

mod pool_context;
mod reports;
mod swaps;

use reports::{
    failed_report, failed_report_at, position_value_from_report, sell_intent_for_position,
    submitted_live_chain_sim_report, unique_order_prefix,
};
use swaps::{simulate_buy_at_block, simulate_sell_at_block};

const LIVE_EXECUTION_DELAY_BLOCKS: u64 = 1;

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
/// Delegates exact-block simulation to chain-server. Alpha keeps the order,
/// pool, and portfolio context needed for strategy decisions, but it does not
/// own or rebuild live EVM state.
#[derive(Clone)]
pub struct LiveChainSimExecutionAdapter {
    chain_server_url: Arc<str>,
    http: reqwest::Client,
    order_prefix: Arc<str>,
    next_order_id: Arc<AtomicU64>,
    current_block: Arc<AtomicU64>,
    current_block_hash: Arc<Mutex<Option<(u64, B256)>>>,
    pools: Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>>,
    portfolio: Arc<Mutex<PortfolioState>>,
    execution_delay_blocks: u64,
}

impl LiveChainSimExecutionAdapter {
    pub fn with_prefix_and_next_order_sequence(
        chain_server_url: impl Into<String>,
        prefix: impl Into<String>,
        next_order_sequence: u64,
    ) -> Result<Self> {
        Ok(Self {
            chain_server_url: Arc::<str>::from(chain_server_url.into().trim_end_matches('/')),
            http: reqwest::Client::new(),
            order_prefix: Arc::<str>::from(prefix.into()),
            next_order_id: Arc::new(AtomicU64::new(next_order_sequence)),
            current_block: Arc::new(AtomicU64::new(0)),
            current_block_hash: Arc::new(Mutex::new(None)),
            pools: Arc::new(Mutex::new(HashMap::new())),
            portfolio: Arc::new(Mutex::new(PortfolioState::default())),
            execution_delay_blocks: LIVE_EXECUTION_DELAY_BLOCKS,
        })
    }

    pub fn current_block(&self) -> Arc<AtomicU64> {
        self.current_block.clone()
    }

    /// Share an externally-owned current-block counter. Used when this adapter
    /// is embedded as a valuation delegate inside the real execution adapter:
    /// the real loop owns the block counter, and valuation must read the same
    /// block the rest of the real path is acting on.
    pub fn with_shared_current_block(mut self, current_block: Arc<AtomicU64>) -> Self {
        self.current_block = current_block;
        self
    }

    pub fn set_current_block_hash(&self, block: Option<u64>, hash: Option<B256>) {
        *self.current_block_hash.lock().expect("block hash lock") = block.zip(hash);
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

    fn current_hash_for_observed_block(&self, observed_block: u64) -> Option<B256> {
        self.current_block_hash
            .lock()
            .expect("block hash lock")
            .and_then(|(block, hash)| (block == observed_block).then_some(hash))
    }

    pub async fn simulate_submitted_order(
        &self,
        order_id: OrderId,
        intent: OrderIntent,
        submitted_block: u64,
        execution_block: u64,
        submitted_block_hash: Option<B256>,
    ) -> Result<Option<ExecutionReport>> {
        let pool = {
            let pools = self.pools.lock().expect("pool lock");
            let Some(pool) = pools.get(&intent.pool_address).cloned() else {
                return Ok(Some(failed_report_at(
                    order_id,
                    "pool not in simulation state",
                    execution_block,
                )));
            };
            pool
        };
        self.simulate_order_request(
            order_id,
            intent,
            pool,
            submitted_block,
            execution_block,
            submitted_block_hash,
            true,
        )
        .await
    }

    async fn simulate_order_request(
        &self,
        order_id: OrderId,
        intent: OrderIntent,
        pool: PoolSnapshot,
        submitted_block: u64,
        execution_block: u64,
        expected_parent_hash: Option<B256>,
        skip_uneconomic_sell: bool,
    ) -> Result<Option<ExecutionReport>> {
        let request = LiveOrderSimulationRequest {
            order_id,
            intent,
            pool,
            submitted_block,
            execution_block,
            expected_block_hash: None,
            expected_parent_hash,
            skip_uneconomic_sell,
        };
        let response = self
            .http
            .post(format!(
                "{}/api/v1/eth/live-tx-simulator/simulations/alpha-order",
                self.chain_server_url
            ))
            .json(&request)
            .send()
            .await
            .map_err(|error| AlphaCoreError::Execution(error.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| AlphaCoreError::Execution(error.to_string()))?;
        if !status.is_success() {
            return Err(AlphaCoreError::Execution(format!(
                "chain-server live order simulation returned HTTP {status}: {body}"
            )));
        }
        let response: LiveOrderSimulationResponse = serde_json::from_str(&body)
            .map_err(|error| AlphaCoreError::Execution(error.to_string()))?;
        if !response.state_available {
            tracing::warn!(
                submitted_block,
                execution_block,
                unavailable_reason = ?response.unavailable_reason,
                selected_block = response.block,
                selected_hash = ?response.block_hash,
                parent_block = ?response.parent_block,
                parent_hash = ?response.parent_block_hash,
                "chain-server live order simulation state unavailable"
            );
            return Ok(None);
        }
        response.report.map(Some).ok_or_else(|| {
            AlphaCoreError::Execution(
                "chain-server live order simulation returned no report".to_string(),
            )
        })
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
            self.current_hash_for_observed_block(observed_block),
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
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-value-{order_seq}", self.order_prefix));
        let Some(report) = self
            .simulate_order_request(
                order_id,
                intent,
                pool.clone(),
                observed_block,
                observed_block,
                None,
                false,
            )
            .await?
        else {
            return Ok(None);
        };
        Ok(position_value_from_report(report, observed_block))
    }
}

#[derive(Debug, Serialize)]
struct LiveOrderSimulationRequest {
    order_id: OrderId,
    intent: OrderIntent,
    pool: PoolSnapshot,
    submitted_block: u64,
    execution_block: u64,
    expected_block_hash: Option<B256>,
    expected_parent_hash: Option<B256>,
    skip_uneconomic_sell: bool,
}

#[derive(Debug, Deserialize)]
struct LiveOrderSimulationResponse {
    #[allow(dead_code)]
    schema: String,
    state_available: bool,
    unavailable_reason: Option<String>,
    block: u64,
    block_hash: Option<B256>,
    parent_block: Option<u64>,
    parent_block_hash: Option<B256>,
    #[allow(dead_code)]
    state_source: String,
    report: Option<ExecutionReport>,
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
