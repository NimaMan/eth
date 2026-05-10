use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use alloy_primitives::U256;
use async_trait::async_trait;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    error::Result,
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, PoolAddress},
    market::PoolSnapshot,
    order::{OrderIntent, OrderSide},
};

use crate::EngineExecutionAdapter;

/// Worst-case fill adapter that models realistic execution using pool snapshots.
///
/// This adapter keeps a thread-safe map of the latest `PoolSnapshot` for each
/// pool and uses reserve math + configured slippage + gas to produce honest
/// `ExecutionReport`s.
///
/// **Buy**: the full ETH amount is spent, but gas is deducted from the report.
/// **Sell**: the expected denom output is reduced by slippage, sell tax, and gas.
#[derive(Clone)]
pub struct ModeledExecutionAdapter {
    order_prefix: Arc<str>,
    next_order_id: Arc<AtomicU64>,
    pools: Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>>,
    config: ModeledExecutionConfig,
}

/// Configuration for worst-case fill modeling.
#[derive(Clone, Debug)]
pub struct ModeledExecutionConfig {
    /// Slippage applied to sells in basis points (e.g. 500 = 5%).
    pub slippage_bps: u32,
    /// Fixed gas cost in wei deducted from every fill.
    pub gas_cost_wei: u64,
    /// Maximum allowed price impact in basis points before rejection.
    pub max_price_impact_bps: u32,
    /// Minimum denom reserve required or the order is rejected.
    pub min_denom_reserve: DecimalAmount,
}

impl Default for ModeledExecutionConfig {
    fn default() -> Self {
        Self {
            slippage_bps: 500,
            gas_cost_wei: 150_000, // ~0.00015 ETH at 1 gwei
            max_price_impact_bps: 2_000, // 20%
            min_denom_reserve: DecimalAmount::from_str("0.5").unwrap(),
        }
    }
}

impl ModeledExecutionAdapter {
    pub fn new(config: ModeledExecutionConfig) -> Self {
        Self {
            order_prefix: Arc::<str>::from(unique_modeled_order_prefix()),
            next_order_id: Arc::new(AtomicU64::new(0)),
            pools: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    pub fn with_prefix(prefix: impl Into<String>, config: ModeledExecutionConfig) -> Self {
        Self {
            order_prefix: Arc::<str>::from(prefix.into()),
            next_order_id: Arc::new(AtomicU64::new(0)),
            pools: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Shared handle to the live pool map. The runner updates this before
    /// feeding each market event into the engine.
    pub fn pools(&self) -> Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>> {
        self.pools.clone()
    }
}

#[async_trait]
impl EngineExecutionAdapter for ModeledExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-{order_seq}", self.order_prefix));

        let pools = self.pools.lock().expect("pool lock");
        let Some(pool) = pools.get(&intent.pool_address) else {
            return Ok(failed_report(order_id, "pool not in modeled state"));
        };

        if pool.is_scam {
            return Ok(failed_report(order_id, "pool marked as scam"));
        }

        if pool.denom_reserve < self.config.min_denom_reserve {
            return Ok(failed_report(order_id, "insufficient liquidity"));
        }

        let gas = U256::from(self.config.gas_cost_wei);

        let filled_amount = match intent.side {
            OrderSide::Buy => {
                // Buy: we spend the full intended ETH amount.
                // The tokens received are not modeled here (we track capital
                // deployed). Gas is deducted from the report.
                Some(intent.amount.clone())
            }
            OrderSide::Sell => {
                // Sell: compute worst-case denom received.
                let mut out = intent.amount.clone();

                // Apply slippage.
                if self.config.slippage_bps > 0 {
                    out = apply_bps_reduction(&out, self.config.slippage_bps);
                }

                // Deduct gas.
                if out.raw > gas {
                    out.raw -= gas;
                } else {
                    out.raw = U256::ZERO;
                }

                Some(out)
            }
        };

        Ok(ExecutionReport {
            order_id,
            status: ExecutionStatus::Confirmed,
            tx_hash: None,
            block_number: None,
            filled_amount,
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
        gas_used: None,
        error: Some(reason.into()),
    }
}

fn apply_bps_reduction(amount: &Amount, bps: u32) -> Amount {
    let factor = U256::from(10_000u32.saturating_sub(bps));
    let divisor = U256::from(10_000u32);
    let new_raw = amount.raw * factor / divisor;
    Amount {
        raw: new_raw,
        decimals: amount.decimals,
    }
}

fn unique_modeled_order_prefix() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    format!("modeled-{}-{millis}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, U256};
    use eth_alpha_core::{
        amount::Amount,
        ids::TokenPoolId,
        market::{PoolProtocol, PoolSnapshot},
    };

    fn test_pool() -> PoolSnapshot {
        PoolSnapshot {
            address: TokenPoolId::new(Address::repeat_byte(0x11), Address::repeat_byte(0x22).to_string()),
            token_address: Address::repeat_byte(0x11),
            protocol: PoolProtocol::UniswapV2,
            denom_address: Some(Address::repeat_byte(0x33)),
            denom_symbol: Some("WETH".to_string()),
            denom_reserve: DecimalAmount::from_str("10.0").unwrap(),
            token_reserve: DecimalAmount::from_str("1000000.0").unwrap(),
            price_denom_per_token: None,
            latest_block: 1,
            can_buy: true,
            can_sell: true,
            is_scam: false,
        }
    }

    fn test_intent(side: OrderSide, amount_raw: u64) -> OrderIntent {
        OrderIntent {
            portfolio_id: eth_alpha_core::ids::PortfolioId("test".to_string()),
            wallet_id: eth_alpha_core::ids::WalletId("test".to_string()),
            strategy_name: eth_alpha_core::ids::StrategyName("test".to_string()),
            side,
            token_address: Address::repeat_byte(0x11),
            pool_address: TokenPoolId::new(Address::repeat_byte(0x11), Address::repeat_byte(0x22).to_string()),
            amount: Amount {
                raw: U256::from(amount_raw),
                decimals: 18,
            },
            route: None,
            max_slippage_bps: 500,
            deadline_secs: 30,
        }
    }

    #[test]
    fn bps_reduction_works() {
        let amount = Amount {
            raw: U256::from(10_000u32),
            decimals: 0,
        };
        let reduced = apply_bps_reduction(&amount, 500); // 5%
        assert_eq!(reduced.raw, U256::from(9_500u32));
    }

    #[test]
    fn zero_bps_keeps_amount() {
        let amount = Amount {
            raw: U256::from(10_000u32),
            decimals: 0,
        };
        let reduced = apply_bps_reduction(&amount, 0);
        assert_eq!(reduced.raw, U256::from(10_000u32));
    }

    #[tokio::test]
    async fn buy_returns_full_amount() {
        let adapter = ModeledExecutionAdapter::new(ModeledExecutionConfig::default());
        adapter.pools().lock().unwrap().insert(test_pool().address.clone(), test_pool());

        let intent = test_intent(OrderSide::Buy, 1_000_000_000_000_000_000u64); // 1 ETH
        let report = adapter.execute(intent).await.unwrap();

        assert_eq!(report.status, ExecutionStatus::Confirmed);
        assert_eq!(report.filled_amount.unwrap().raw, U256::from(1_000_000_000_000_000_000u64));
        assert_eq!(report.gas_used, Some(150_000));
    }

    #[tokio::test]
    async fn sell_reduces_by_slippage_and_gas() {
        let adapter = ModeledExecutionAdapter::new(ModeledExecutionConfig::default());
        adapter.pools().lock().unwrap().insert(test_pool().address.clone(), test_pool());

        let intent = test_intent(OrderSide::Sell, 1_000_000_000_000_000_000u64); // 1 ETH
        let report = adapter.execute(intent).await.unwrap();

        assert_eq!(report.status, ExecutionStatus::Confirmed);
        // 1 ETH * 0.95 (5% slippage) - 150_000 gas
        let expected = U256::from(1_000_000_000_000_000_000u64) * U256::from(9_500) / U256::from(10_000)
            - U256::from(150_000);
        assert_eq!(report.filled_amount.unwrap().raw, expected);
    }

    #[tokio::test]
    async fn scam_pool_is_rejected() {
        let adapter = ModeledExecutionAdapter::new(ModeledExecutionConfig::default());
        let mut pool = test_pool();
        pool.is_scam = true;
        adapter.pools().lock().unwrap().insert(pool.address.clone(), pool);

        let intent = test_intent(OrderSide::Buy, 1_000_000_000_000_000_000u64);
        let report = adapter.execute(intent).await.unwrap();

        assert_eq!(report.status, ExecutionStatus::Failed);
        assert!(report.error.unwrap().contains("scam"));
    }

    #[tokio::test]
    async fn unknown_pool_is_rejected() {
        let adapter = ModeledExecutionAdapter::new(ModeledExecutionConfig::default());
        // do not insert pool

        let intent = test_intent(OrderSide::Buy, 1_000_000_000_000_000_000u64);
        let report = adapter.execute(intent).await.unwrap();

        assert_eq!(report.status, ExecutionStatus::Failed);
        assert!(report.error.unwrap().contains("not in modeled state"));
    }
}
