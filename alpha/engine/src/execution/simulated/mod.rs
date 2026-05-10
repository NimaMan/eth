//! EVM-backed execution adapters.
//!
//! Two variants are provided:
//!
//! | Adapter | Use Case | Block Source |
//! |---------|----------|-------------|
//! | `SimulatedExecutionAdapter` | Historical backtest | Manually-set `current_block` |
//! | `LiveSimulatedExecutionAdapter` | Live/paper trading | `LiveTxSimulator::latest_state_block_number()` |
//!
//! Both run the actual swap calldata through the EVM so that token taxes, max
//! transaction limits, and other contract-level behaviour are captured exactly.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use alloy_primitives::Address;
use async_trait::async_trait;
use eth_alpha_core::{
    amount::Amount,
    error::Result,
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, PoolAddress},
    market::{PoolProtocol, PoolSnapshot},
    order::{OrderIntent, OrderSide},
    portfolio::PortfolioState,
};
use tx_processor::{
    simulate_buy_swap, simulate_sell_swap, BuySwapResult, PoolType, SellSwapResult,
};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::{LiveTxSimulator, TxSimulator};

use crate::EngineExecutionAdapter;

// ---------------------------------------------------------------------------
// Shared simulation logic (private)
// ---------------------------------------------------------------------------

/// Simulate a buy at a specific block.  Returns the ExecutionReport with
/// `token_amount` populated from the EVM trace.
async fn simulate_buy_at_block(
    simulator: &Arc<TxSimulator>,
    tx_processor: &Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: &PoolSnapshot,
    pool_type: PoolType,
    block: u64,
) -> Result<ExecutionReport> {
    let eth_amount = intent.amount.raw;
    let pool_contract_address = parse_pool_address(&pool.address)?;

    let result: BuySwapResult = simulate_buy_swap(
        simulator.clone(),
        tx_processor.clone(),
        intent.token_address,
        pool_contract_address,
        pool_type,
        Some(block),
        eth_amount,
    )
    .await
    .map_err(|e| eth_alpha_core::error::AlphaCoreError::Execution(e.to_string()))?;

    if !result.success {
        return Ok(failed_report(
            order_id,
            result.failure_reason.unwrap_or("buy simulation failed".to_string()),
        ));
    }

    // Token decimals default to 18 if unknown.
    let token_decimals = 18u8;
    let token_amount = Amount {
        raw: result.tokens_received,
        decimals: token_decimals,
    };

    Ok(ExecutionReport {
        order_id,
        status: ExecutionStatus::Confirmed,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: Some(intent.amount),
        token_amount: Some(token_amount),
        gas_used: Some(result.buy_transaction.fees.gas_used),
        error: None,
    })
}

/// Simulate a sell at a specific block.  Looks up `entry_token_amount` from the
/// open position, converts to raw U256, and simulates the swap.
async fn simulate_sell_at_block(
    simulator: &Arc<TxSimulator>,
    tx_processor: &Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: &PoolSnapshot,
    pool_type: PoolType,
    block: u64,
    _portfolio: &Arc<Mutex<PortfolioState>>,
) -> Result<ExecutionReport> {
    // intent.amount is the token quantity to sell (raw U256 scaled by token decimals).
    let tokens_to_sell = intent.amount.raw;

    if tokens_to_sell.is_zero() {
        return Ok(failed_report(order_id, "zero token amount; nothing to sell"));
    }

    let pool_contract_address = parse_pool_address(&pool.address)?;

    let result: SellSwapResult = simulate_sell_swap(
        simulator.clone(),
        tx_processor.clone(),
        intent.token_address,
        pool_contract_address,
        pool_type,
        tokens_to_sell,
        Some(block),
    )
    .await
    .map_err(|e| eth_alpha_core::error::AlphaCoreError::Execution(e.to_string()))?;

    if !result.success {
        return Ok(failed_report(
            order_id,
            result.failure_reason.unwrap_or("sell simulation failed".to_string()),
        ));
    }

    let denom_received = Amount {
        raw: result.denom_received,
        decimals: 18,
    };

    Ok(ExecutionReport {
        order_id,
        status: ExecutionStatus::Confirmed,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: Some(denom_received),
        token_amount: None,
        gas_used: Some(result.sell_transaction.fees.gas_used),
        error: None,
    })
}

// ---------------------------------------------------------------------------
// Historical backtest adapter
// ---------------------------------------------------------------------------

/// Historical simulation adapter.
///
/// Runs swaps against a specific block number that is set externally by the
/// backtest runner.  Use this when replaying historical observations.
#[derive(Clone)]
pub struct SimulatedExecutionAdapter {
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    order_prefix: Arc<str>,
    next_order_id: Arc<AtomicU64>,
    pools: Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>>,
    portfolio: Arc<Mutex<PortfolioState>>,
    current_block: Arc<AtomicU64>,
}

impl SimulatedExecutionAdapter {
    pub fn new(
        simulator: Arc<TxSimulator>,
        tx_processor: Arc<TxProcessor>,
    ) -> Result<Self> {
        Ok(Self {
            simulator,
            tx_processor,
            order_prefix: Arc::<str>::from(unique_order_prefix()),
            next_order_id: Arc::new(AtomicU64::new(0)),
            pools: Arc::new(Mutex::new(HashMap::new())),
            portfolio: Arc::new(Mutex::new(PortfolioState::default())),
            current_block: Arc::new(AtomicU64::new(0)),
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
        })
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
impl EngineExecutionAdapter for SimulatedExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-{order_seq}", self.order_prefix));

        let (pool, pool_type, block) = {
            let pools = self.pools.lock().expect("pool lock");
            let Some(pool) = pools.get(&intent.pool_address).cloned() else {
                return Ok(failed_report(order_id, "pool not in simulation state"));
            };

            let pool_type = match pool_protocol_to_tx_processor(pool.protocol.clone()) {
                Some(pt) => pt,
                None => {
                    return Ok(failed_report(
                        order_id,
                        format!("protocol {:?} not supported by simulator", pool.protocol),
                    ));
                }
            };

            let block = self.current_block.load(Ordering::Relaxed);
            (pool, pool_type, block)
        };

        if block == 0 {
            return Ok(failed_report(order_id, "current block not set"));
        }

        match intent.side {
            OrderSide::Buy => {
                simulate_buy_at_block(
                    &self.simulator,
                    &self.tx_processor,
                    order_id,
                    intent,
                    &pool,
                    pool_type,
                    block,
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
                    pool_type,
                    block,
                    &self.portfolio,
                )
                .await
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Live trading adapter
// ---------------------------------------------------------------------------

/// Live simulation adapter.
///
/// Uses `LiveTxSimulator` so that every fill runs against the latest tracked
/// chain state (local Reth context when caught up, otherwise live block
/// processor state).  The block number is fetched automatically on each
/// execution — no manual `current_block` updates required.
#[derive(Clone)]
pub struct LiveSimulatedExecutionAdapter {
    live_sim: LiveTxSimulator,
    tx_processor: Arc<TxProcessor>,
    order_prefix: Arc<str>,
    next_order_id: Arc<AtomicU64>,
    pools: Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>>,
    portfolio: Arc<Mutex<PortfolioState>>,
}

impl LiveSimulatedExecutionAdapter {
    pub fn new(
        live_sim: LiveTxSimulator,
        tx_processor: Arc<TxProcessor>,
    ) -> Result<Self> {
        Ok(Self {
            live_sim,
            tx_processor,
            order_prefix: Arc::<str>::from(unique_order_prefix()),
            next_order_id: Arc::new(AtomicU64::new(0)),
            pools: Arc::new(Mutex::new(HashMap::new())),
            portfolio: Arc::new(Mutex::new(PortfolioState::default())),
        })
    }

    pub fn with_prefix(
        live_sim: LiveTxSimulator,
        tx_processor: Arc<TxProcessor>,
        prefix: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            live_sim,
            tx_processor,
            order_prefix: Arc::<str>::from(prefix.into()),
            next_order_id: Arc::new(AtomicU64::new(0)),
            pools: Arc::new(Mutex::new(HashMap::new())),
            portfolio: Arc::new(Mutex::new(PortfolioState::default())),
        })
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
}

#[async_trait]
impl EngineExecutionAdapter for LiveSimulatedExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-{order_seq}", self.order_prefix));

        let (pool, pool_type) = {
            let pools = self.pools.lock().expect("pool lock");
            let Some(pool) = pools.get(&intent.pool_address).cloned() else {
                return Ok(failed_report(order_id, "pool not in simulation state"));
            };

            let pool_type = match pool_protocol_to_tx_processor(pool.protocol.clone()) {
                Some(pt) => pt,
                None => {
                    return Ok(failed_report(
                        order_id,
                        format!("protocol {:?} not supported by simulator", pool.protocol),
                    ));
                }
            };
            (pool, pool_type)
        };

        let block = self
            .live_sim
            .latest_state_block_number()
            .await
            .map_err(|e| eth_alpha_core::error::AlphaCoreError::Execution(e.to_string()))?;

        let simulator = self.live_sim.simulator();

        match intent.side {
            OrderSide::Buy => {
                simulate_buy_at_block(
                    &simulator,
                    &self.tx_processor,
                    order_id,
                    intent,
                    &pool,
                    pool_type,
                    block,
                )
                .await
            }
            OrderSide::Sell => {
                simulate_sell_at_block(
                    &simulator,
                    &self.tx_processor,
                    order_id,
                    intent,
                    &pool,
                    pool_type,
                    block,
                    &self.portfolio,
                )
                .await
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

fn unique_order_prefix() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    format!("sim-{}-{millis}", std::process::id())
}

/// Map alpha `PoolProtocol` to tx_processor `PoolType`.
fn pool_protocol_to_tx_processor(protocol: PoolProtocol) -> Option<PoolType> {
    match protocol {
        PoolProtocol::UniswapV2 => Some(PoolType::UniswapV2),
        PoolProtocol::UniswapV3 => {
            // Default to 0.3% fee tier when unknown.
            Some(PoolType::UniswapV3 { fee_tier: 3000 })
        }
        PoolProtocol::UniswapV4 => Some(PoolType::UniswapV4),
        PoolProtocol::Unknown(ref s) => match s.as_str() {
            "sushi" | "sushiswap" => Some(PoolType::SushiSwap),
            "pancake" | "pancakeswap" => Some(PoolType::PancakeSwapV2),
            _ => None,
        },
    }
}

/// Parse the pool contract address from a `TokenPoolId`.
fn parse_pool_address(pool_id: &PoolAddress) -> Result<Address> {
    let s = pool_id.as_str();
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 {
        return Err(eth_alpha_core::error::AlphaCoreError::InvalidPoolAddress(s.to_string()));
    }
    let pool_part = parts[1];
    let addr_str = pool_part.split('#').next().unwrap_or(pool_part);
    addr_str
        .parse::<Address>()
        .map_err(|_| eth_alpha_core::error::AlphaCoreError::InvalidPoolAddress(s.to_string()))
}


