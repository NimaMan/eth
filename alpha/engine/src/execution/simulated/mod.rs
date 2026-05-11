//! Chain-state EVM execution adapters.
//!
//! Two variants are provided:
//!
//! | Adapter | Use Case | Block Source |
//! |---------|----------|-------------|
//! | `ChainSimExecutionAdapter` | Historical backtest | Manually-set `current_block` |
//! | `LiveChainSimExecutionAdapter` | Live no-capital trading | `LiveTxSimulator::latest_state_block_number()` |
//!
//! Both run the actual swap calldata through the EVM so that token taxes, max
//! transaction limits, and other contract-level behaviour are captured from
//! chain state. No snapshot-price or perfect-fill fallback is allowed here.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use alloy_primitives::{address, Address, Bytes, U256};
use async_trait::async_trait;
use eth_alpha_core::{
    amount::Amount,
    error::Result,
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, PoolAddress},
    market::{PoolProtocol, PoolSnapshot},
    order::{OrderIntent, OrderSide},
    portfolio::PortfolioState,
    position::Position,
};
use tx_processor::tx_processor::TxProcessor;
use tx_processor::{
    simulate_buy_swap_with_params, simulate_sell_swap_with_params, BuySwapResult,
    PoolBuySellParameters, PoolType, SellSwapResult, UniswapV4PoolConfig as TxUniswapV4PoolConfig,
};
use tx_simulator::{LiveTxSimulator, TxSimulator};

use crate::{EngineExecutionAdapter, PositionValueSimulation};

const ERC20_DECIMALS_SELECTOR: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];
const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
const USDC_ADDRESS: Address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
const USDT_ADDRESS: Address = address!("dAC17F958D2ee523a2206206994597C13D831ec7");
const DAI_ADDRESS: Address = address!("6B175474E89094C44Da98b954EedeAC495271d0F");

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
    block: u64,
) -> Result<ExecutionReport> {
    let eth_amount = intent.amount.raw;
    let params =
        match pool_simulation_parameters(simulator, pool, intent.token_address, eth_amount, block)
            .await
        {
            Ok(params) => params,
            Err(error) => {
                return Ok(failed_report(
                    order_id,
                    format!("invalid pool parameters for chain simulation: {error}"),
                ))
            }
        };

    let result: BuySwapResult = match simulate_buy_swap_with_params(
        simulator.clone(),
        tx_processor.clone(),
        params.clone(),
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return Ok(failed_report(
                order_id,
                format!("chain buy simulation failed: {error}"),
            ))
        }
    };

    if !result.success {
        return Ok(failed_report(
            order_id,
            result
                .failure_reason
                .unwrap_or("buy simulation failed".to_string()),
        ));
    }

    let token_amount = Amount {
        raw: result.tokens_received,
        decimals: params.token_decimals,
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

/// Simulate a sell at a specific block using a raw token amount and the same
/// swap simulation path used for exits and mark-to-market valuation.
async fn simulate_sell_at_block(
    simulator: &Arc<TxSimulator>,
    tx_processor: &Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: &PoolSnapshot,
    block: u64,
    _portfolio: &Arc<Mutex<PortfolioState>>,
) -> Result<ExecutionReport> {
    // intent.amount is the token quantity to sell (raw U256 scaled by token decimals).
    let tokens_to_sell = intent.amount.raw;

    if tokens_to_sell.is_zero() {
        return Ok(failed_report(
            order_id,
            "zero token amount; nothing to sell",
        ));
    }

    let params = match pool_simulation_parameters(
        simulator,
        pool,
        intent.token_address,
        tokens_to_sell,
        block,
    )
    .await
    {
        Ok(params) => params,
        Err(error) => {
            return Ok(failed_report(
                order_id,
                format!("invalid pool parameters for chain simulation: {error}"),
            ))
        }
    };

    let result: SellSwapResult = match simulate_sell_swap_with_params(
        simulator.clone(),
        tx_processor.clone(),
        params,
        tokens_to_sell,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return Ok(failed_report(
                order_id,
                format!("chain sell simulation failed: {error}"),
            ))
        }
    };

    if !result.success {
        return Ok(failed_report(
            order_id,
            result
                .failure_reason
                .unwrap_or("sell simulation failed".to_string()),
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
impl EngineExecutionAdapter for ChainSimExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-{order_seq}", self.order_prefix));

        let (pool, block) = {
            let pools = self.pools.lock().expect("pool lock");
            let Some(pool) = pools.get(&intent.pool_address).cloned() else {
                return Ok(failed_report(order_id, "pool not in simulation state"));
            };

            let block = self.current_block.load(Ordering::Relaxed);
            (pool, block)
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
                    block,
                    &self.portfolio,
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
/// Uses `LiveTxSimulator` so that every fill runs against the latest tracked
/// chain state (local Reth context when caught up, otherwise live block
/// processor state).  The block number is fetched automatically on each
/// execution — no manual `current_block` updates required.
#[derive(Clone)]
pub struct LiveChainSimExecutionAdapter {
    live_sim: LiveTxSimulator,
    tx_processor: Arc<TxProcessor>,
    order_prefix: Arc<str>,
    next_order_id: Arc<AtomicU64>,
    pools: Arc<Mutex<HashMap<PoolAddress, PoolSnapshot>>>,
    portfolio: Arc<Mutex<PortfolioState>>,
}

impl LiveChainSimExecutionAdapter {
    pub fn new(live_sim: LiveTxSimulator, tx_processor: Arc<TxProcessor>) -> Result<Self> {
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
                    block,
                    &self.portfolio,
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
        let block = match self.live_sim.latest_state_block_number().await {
            Ok(block) => block,
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "chain-sim position valuation skipped because no live state block is available"
                );
                return Ok(None);
            }
        };
        let simulator = self.live_sim.simulator();
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        let order_id = OrderId(format!("{}-value-{order_seq}", self.order_prefix));
        let report = simulate_sell_at_block(
            &simulator,
            &self.tx_processor,
            order_id,
            intent,
            pool,
            block,
            &self.portfolio,
        )
        .await?;
        Ok(position_value_from_report(report, block))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn pool_simulation_parameters(
    simulator: &Arc<TxSimulator>,
    pool: &PoolSnapshot,
    token_address: Address,
    amount: U256,
    block: u64,
) -> std::result::Result<PoolBuySellParameters, String> {
    let pool_contract_address = parse_pool_address(&pool.address)
        .map_err(|error| format!("invalid pool address for chain simulation: {error}"))?;
    let pool_type = pool_type_for_pool(pool).ok_or_else(|| {
        format!(
            "protocol {:?} not supported by chain simulator",
            pool.protocol
        )
    })?;
    let token_decimals = match pool.token_decimals {
        Some(decimals) => decimals,
        None => query_erc20_decimals(simulator, token_address, block).await?,
    };
    let denom_address = pool
        .denom_address
        .ok_or_else(|| format!("pool {} has no denomination address", pool.address))?;
    let denom_decimals = denom_decimals(simulator, denom_address, block).await?;

    let mut params = PoolBuySellParameters::new(token_address, pool_contract_address, pool_type)
        .with_test_amount(amount)
        .with_block(block)
        .with_denom_address(denom_address)
        .with_denom_decimals(denom_decimals)
        .with_token_decimals(token_decimals);

    if let Some(v4) = &pool.uniswap_v4 {
        params = params.with_uniswap_v4_config(TxUniswapV4PoolConfig {
            pool_manager: v4.pool_manager,
            pool_id: v4.pool_id,
            currency0: v4.currency0,
            currency1: v4.currency1,
            fee: v4.fee,
            tick_spacing: v4.tick_spacing,
            hooks: v4.hooks,
            hook_data: Vec::new(),
        });
    }

    Ok(params)
}

fn pool_type_for_pool(pool: &PoolSnapshot) -> Option<PoolType> {
    match &pool.protocol {
        PoolProtocol::UniswapV2 => Some(PoolType::UniswapV2),
        PoolProtocol::UniswapV3 => Some(PoolType::UniswapV3 {
            fee_tier: pool.fee_tier.unwrap_or(3000),
        }),
        PoolProtocol::UniswapV4 => Some(PoolType::UniswapV4),
        PoolProtocol::Unknown(s) => match s.as_str() {
            "sushi" | "sushiswap" => Some(PoolType::SushiSwap),
            "pancake" | "pancakeswap" => Some(PoolType::PancakeSwapV2),
            _ => None,
        },
    }
}

async fn denom_decimals(
    simulator: &Arc<TxSimulator>,
    denom_address: Address,
    block: u64,
) -> std::result::Result<u8, String> {
    if denom_address.is_zero() || denom_address == WETH_ADDRESS || denom_address == DAI_ADDRESS {
        return Ok(18);
    }
    if denom_address == USDC_ADDRESS || denom_address == USDT_ADDRESS {
        return Ok(6);
    }
    query_erc20_decimals(simulator, denom_address, block).await
}

async fn query_erc20_decimals(
    simulator: &Arc<TxSimulator>,
    token_address: Address,
    block: u64,
) -> std::result::Result<u8, String> {
    let result = simulator
        .simulate_view_function(
            token_address,
            Bytes::from_static(&ERC20_DECIMALS_SELECTOR),
            Some(block),
        )
        .await
        .map_err(|error| format!("token decimals simulation failed: {error}"))?;
    if !result.success {
        return Err("token decimals simulation reverted".to_string());
    }
    if result.output.len() < 32 {
        return Err(format!(
            "token decimals simulation returned short output: {} bytes",
            result.output.len()
        ));
    }
    Ok(result.decode_uint8())
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

fn sell_intent_for_position(position: &Position) -> Option<OrderIntent> {
    Some(OrderIntent {
        portfolio_id: position.key.portfolio_id.clone(),
        wallet_id: position.key.wallet_id.clone(),
        strategy_name: position.key.strategy_name.clone(),
        side: OrderSide::Sell,
        token_address: position.key.token_address,
        pool_address: position.key.pool_address.clone(),
        amount: position.entry_token_raw_amount.clone()?,
        route: None,
        max_slippage_bps: 0,
        deadline_secs: 0,
    })
}

fn position_value_from_report(
    report: ExecutionReport,
    block_number: u64,
) -> Option<PositionValueSimulation> {
    match report.status {
        ExecutionStatus::Confirmed => {
            let current_value = report.filled_amount?;
            Some(PositionValueSimulation {
                block_number: report.block_number.unwrap_or(block_number),
                current_value,
                gas_used: report.gas_used,
                error: None,
            })
        }
        ExecutionStatus::Failed => {
            let error = report
                .error
                .unwrap_or_else(|| "chain-sim position valuation failed".to_string());
            if valuation_was_unavailable(&error) {
                return None;
            }
            Some(PositionValueSimulation {
                block_number,
                current_value: Amount::zero(18),
                gas_used: report.gas_used,
                error: Some(error),
            })
        }
        ExecutionStatus::Submitted | ExecutionStatus::Pending | ExecutionStatus::Cancelled => None,
    }
}

fn valuation_was_unavailable(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("not supported")
        || lower.contains("invalid pool address")
        || lower.contains("current block not set")
        || lower.contains("pool not in simulation state")
        || lower.contains("unable to inject synthetic erc20 balance")
        || lower.contains("state for block")
}

fn unique_order_prefix() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    format!("chain-sim-{}-{millis}", std::process::id())
}

/// Parse the pool contract address from a `TokenPoolId`.
fn parse_pool_address(pool_id: &PoolAddress) -> Result<Address> {
    let s = pool_id.as_str();
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 {
        return Err(eth_alpha_core::error::AlphaCoreError::InvalidPoolAddress(
            s.to_string(),
        ));
    }
    let pool_part = parts[1];
    let addr_str = pool_part.split('#').next().unwrap_or(pool_part);
    addr_str
        .parse::<Address>()
        .map_err(|_| eth_alpha_core::error::AlphaCoreError::InvalidPoolAddress(s.to_string()))
}
