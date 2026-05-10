use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use alloy_primitives::U256;
use async_trait::async_trait;
use eth_alpha_core::{
    amount::Amount,
    error::Result,
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, PoolAddress},
    market::PoolSnapshot,
    order::{OrderIntent, OrderSide},
};
use eth_alpha_engine::EngineExecutionAdapter;

use crate::config::SimulationConfig;

/// Simulated execution adapter that models block-level fills with explicit
/// pessimistic assumptions.
///
/// The adapter maintains its own view of the current pool snapshots so that it
/// can look up prices and liquidity when the engine asks it to execute an
/// intent.  Because the state is held behind `Arc` handles, a clone of the
/// adapter shares the same state — this lets the backtest runner update the
/// pool map before each event while the engine holds the original handle.
#[derive(Clone)]
pub struct SimulatedExecutionAdapter {
    pools: Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>>,
    current_block: Arc<AtomicU64>,
    config: SimulationConfig,
    order_seq: Arc<AtomicU64>,
    run_id: Arc<str>,
}

impl SimulatedExecutionAdapter {
    pub fn new(config: SimulationConfig, run_id: impl Into<String>) -> Self {
        Self {
            pools: Arc::new(Mutex::new(HashMap::new())),
            current_block: Arc::new(AtomicU64::new(0)),
            config,
            order_seq: Arc::new(AtomicU64::new(0)),
            run_id: Arc::<str>::from(run_id.into()),
        }
    }

    /// Shared handle to the live pool map.  The runner updates this before
    /// feeding each event into the engine.
    pub fn pools(&self) -> Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>> {
        self.pools.clone()
    }

    /// Shared handle to the current block number.
    pub fn current_block(&self) -> Arc<AtomicU64> {
        self.current_block.clone()
    }
}

#[async_trait]
impl EngineExecutionAdapter for SimulatedExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let order_seq = self.order_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-{order_seq}", self.run_id));

        let pools = self.pools.lock().expect("pool lock");
        let Some(pool) = pools.get(&intent.pool_address) else {
            return Ok(failed_report(order_id, "pool not in simulation state"));
        };

        if self.config.reject_scam && pool.is_scam {
            return Ok(failed_report(order_id, "pool marked as scam"));
        }

        if self.config.reject_insufficient_liquidity
            && pool.denom_reserve < self.config.min_denom_reserve
        {
            return Ok(failed_report(order_id, "insufficient liquidity"));
        }

        if self.config.failure_rate_bps > 0 {
            let roll = rand::random::<u32>() % 10_000;
            if roll < self.config.failure_rate_bps {
                return Ok(failed_report(order_id, "simulated random failure"));
            }
        }

        let filled_amount = match intent.side {
            OrderSide::Buy => {
                // Worst-case buy: the intended ETH amount is spent in full.
                // filled_amount in the current schema tracks capital deployed.
                Some(intent.amount.clone())
            }
            OrderSide::Sell => {
                // Sell: intent.amount is tokens to sell.
                // Compute ETH received = tokens * price, then apply slippage.
                let token_qty = intent.amount.to_decimal();
                let price = pool.price_denom_per_token.unwrap_or_default();
                let mut eth = token_qty * price;

                if self.config.worst_case_fill && self.config.slippage_bps > 0 {
                    let factor = eth_alpha_core::amount::DecimalAmount::from(
                        10_000i64 - i64::from(self.config.slippage_bps),
                    ) / eth_alpha_core::amount::DecimalAmount::from(10_000i64);
                    eth = eth * factor;
                }

                let eth_amount = eth_alpha_core::amount::Amount::from_decimal(eth, 18);
                Some(eth_amount)
            }
        };

        Ok(ExecutionReport {
            order_id,
            status: ExecutionStatus::Confirmed,
            tx_hash: None,
            block_number: Some(self.current_block.load(Ordering::Relaxed)),
            filled_amount,
            token_amount: None,
            gas_used: Some(self.config.gas_cost_wei),
            error: None,
        })
    }
}

fn failed_report(order_id: OrderId, reason: impl Into<String>) -> ExecutionReport {
    ExecutionReport {
        order_id,
        status: ExecutionStatus::Failed,
        tx_hash: None,
        block_number: None,
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        error: Some(reason.into()),
    }
}

/// Reduce an amount by `slippage_bps / 10_000`.
fn apply_slippage(amount: &Amount, slippage_bps: u32) -> Amount {
    let factor = U256::from(10_000u32.saturating_sub(slippage_bps));
    let divisor = U256::from(10_000u32);
    let new_raw = amount.raw * factor / divisor;
    Amount {
        raw: new_raw,
        decimals: amount.decimals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;

    #[test]
    fn slippage_reduces_amount() {
        let amount = Amount {
            raw: U256::from(10_000u32),
            decimals: 0,
        };
        let adjusted = apply_slippage(&amount, 100); // 1%
        assert_eq!(adjusted.raw, U256::from(9_900u32));
    }

    #[test]
    fn zero_slippage_keeps_amount() {
        let amount = Amount {
            raw: U256::from(10_000u32),
            decimals: 0,
        };
        let adjusted = apply_slippage(&amount, 0);
        assert_eq!(adjusted.raw, U256::from(10_000u32));
    }
}
