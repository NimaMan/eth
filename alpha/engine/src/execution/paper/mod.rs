use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use eth_alpha_core::{
    amount::Amount,
    error::Result,
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, PoolAddress},
    market::PoolSnapshot,
    order::{OrderIntent, OrderSide},
};

use crate::EngineExecutionAdapter;

/// Perfect-fill paper adapter.
///
/// Buy: spends the full ETH amount, receives tokens (tracked via `token_amount`).
/// Sell: sells the specified token amount, receives ETH = token_qty * price.
/// Gas is always zero. Useful for rapid prototyping.
#[derive(Clone)]
pub struct PaperExecutionAdapter {
    order_prefix: Arc<str>,
    next_order_id: Arc<AtomicU64>,
    pools: Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>>,
}

impl PaperExecutionAdapter {
    pub fn new() -> Self {
        Self::with_order_prefix(unique_paper_order_prefix())
    }

    pub fn with_order_prefix(prefix: impl Into<String>) -> Self {
        Self {
            order_prefix: Arc::<str>::from(prefix.into()),
            next_order_id: Arc::new(AtomicU64::new(0)),
            pools: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn pools(&self) -> Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>> {
        self.pools.clone()
    }
}

impl Default for PaperExecutionAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EngineExecutionAdapter for PaperExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-{order_seq}", self.order_prefix));

        match intent.side {
            OrderSide::Buy => {
                // Buy: ETH spent = intent.amount, tokens received not modeled.
                Ok(ExecutionReport {
                    order_id,
                    status: ExecutionStatus::Confirmed,
                    tx_hash: None,
                    block_number: None,
                    filled_amount: Some(intent.amount),
                    token_amount: None,
                    gas_used: Some(0),
                    error: None,
                })
            }
            OrderSide::Sell => {
                // Sell: intent.amount is tokens. Compute ETH = tokens * price.
                let pools = self.pools.lock().expect("pool lock");
                let eth_received = if let Some(pool) = pools.get(&intent.pool_address) {
                    let token_qty = intent.amount.to_decimal();
                    let price = pool.price_denom_per_token.unwrap_or_default();
                    let eth = token_qty * price;
                    Amount::from_decimal(eth, 18)
                } else {
                    Amount::zero(18)
                };
                Ok(ExecutionReport {
                    order_id,
                    status: ExecutionStatus::Confirmed,
                    tx_hash: None,
                    block_number: None,
                    filled_amount: Some(eth_received),
                    token_amount: None,
                    gas_used: Some(0),
                    error: None,
                })
            }
        }
    }
}

fn unique_paper_order_prefix() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("paper-{}-{millis}", std::process::id())
}
