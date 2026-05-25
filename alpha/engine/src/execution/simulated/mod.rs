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
use tokio::time::sleep;
use tx_processor::tx_processor::TxProcessor;
use tx_processor::{
    simulate_buy_swap_with_params, simulate_buy_swap_with_params_and_chain,
    simulate_sell_swap_with_params, simulate_sell_swap_with_params_and_chain, BuySwapResult,
    PoolBuySellParameters, PoolType, SellSwapResult, UniswapV4PoolConfig as TxUniswapV4PoolConfig,
};
use tx_simulator::{LiveTxSimulator, TxSimulator, UnsignedTxChainSimulation};

use crate::{
    execution::sell_economics::uneconomic_sell_cancellation_reason, EngineExecutionAdapter,
    PositionValueSimulation,
};

const ERC20_DECIMALS_SELECTOR: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];
const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
const USDC_ADDRESS: Address = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
const USDT_ADDRESS: Address = address!("dAC17F958D2ee523a2206206994597C13D831ec7");
const DAI_ADDRESS: Address = address!("6B175474E89094C44Da98b954EedeAC495271d0F");
const LIVE_EXECUTION_DELAY_BLOCKS: u64 = 1;
const LIVE_STATE_WAIT_TIMEOUT: Duration = Duration::from_secs(45);
const LIVE_STATE_WAIT_INTERVAL: Duration = Duration::from_millis(500);

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
                return Ok(failed_report_at(
                    order_id,
                    format!("invalid pool parameters for chain simulation: {error}"),
                    block,
                ));
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
            return Ok(failed_report_at(
                order_id,
                format!("chain buy simulation failed: {error}"),
                block,
            ));
        }
    };

    if !result.success {
        return Ok(failed_report_at_with_gas(
            order_id,
            result
                .failure_reason
                .unwrap_or("buy simulation failed".to_string()),
            block,
            Some(result.buy_transaction.fees.gas_used),
            Some(result.buy_transaction.fees.tx_fee),
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
        gas_cost: Some(gas_cost_amount(result.buy_transaction.fees.tx_fee)),
        mined_evidence: None,
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
    skip_uneconomic_sell: bool,
) -> Result<ExecutionReport> {
    // intent.amount is the token quantity to sell (raw U256 scaled by token decimals).
    let tokens_to_sell = intent.amount.raw;

    if tokens_to_sell.is_zero() {
        return Ok(failed_report_at(
            order_id,
            "zero token amount; nothing to sell",
            block,
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
            return Ok(failed_report_at(
                order_id,
                format!("invalid pool parameters for chain simulation: {error}"),
                block,
            ));
        }
    };

    let denom_decimals = params.denom_decimals;
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
            return Ok(failed_report_at(
                order_id,
                format!("chain sell simulation failed: {error}"),
                block,
            ));
        }
    };

    if !result.success {
        return Ok(failed_report_at_with_gas(
            order_id,
            result
                .failure_reason
                .unwrap_or("sell simulation failed".to_string()),
            block,
            Some(result.gas_used),
            Some(result.gas_cost),
        ));
    }

    let denom_received = Amount {
        raw: result.denom_received,
        decimals: denom_decimals,
    };

    if skip_uneconomic_sell {
        if let Some(reason) =
            uneconomic_sell_cancellation_reason(pool, result.denom_received, result.gas_cost)
        {
            return Ok(cancelled_report_at(order_id, reason, block));
        }
    }

    Ok(ExecutionReport {
        order_id,
        status: ExecutionStatus::Confirmed,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: Some(denom_received),
        token_amount: None,
        gas_used: Some(result.gas_used),
        gas_cost: Some(gas_cost_amount(result.gas_cost)),
        mined_evidence: None,
        error: None,
    })
}

async fn simulate_live_buy_at_block(
    live_simulator: &LiveTxSimulator,
    tx_processor: &Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: &PoolSnapshot,
    block: u64,
) -> Result<ExecutionReport> {
    let simulator = live_simulator.simulator();
    let eth_amount = intent.amount.raw;
    let mut chain = match live_simulator.start_chain_at(block).await {
        Ok(chain) => chain,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("live simulator state unavailable for buy block {block}: {error}"),
                block,
            ));
        }
    };
    let params = match pool_simulation_parameters_live(
        &mut chain,
        pool,
        intent.token_address,
        eth_amount,
        block,
    ) {
        Ok(params) => params,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("invalid pool parameters for live chain simulation: {error}"),
                block,
            ));
        }
    };
    let token_decimals = params.token_decimals;
    let result: BuySwapResult = match simulate_buy_swap_with_params_and_chain(
        simulator,
        tx_processor.clone(),
        params,
        chain,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("live chain buy simulation failed: {error}"),
                block,
            ));
        }
    };
    if !result.success {
        return Ok(failed_report_at_with_gas(
            order_id,
            result
                .failure_reason
                .unwrap_or("buy simulation failed".to_string()),
            block,
            Some(result.buy_transaction.fees.gas_used),
            Some(result.buy_transaction.fees.tx_fee),
        ));
    }

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
        gas_cost: Some(gas_cost_amount(result.buy_transaction.fees.tx_fee)),
        mined_evidence: None,
        error: None,
    })
}

async fn simulate_live_sell_at_block(
    live_simulator: &LiveTxSimulator,
    tx_processor: &Arc<TxProcessor>,
    order_id: OrderId,
    intent: OrderIntent,
    pool: &PoolSnapshot,
    block: u64,
    _portfolio: &Arc<Mutex<PortfolioState>>,
    skip_uneconomic_sell: bool,
) -> Result<ExecutionReport> {
    let simulator = live_simulator.simulator();
    let tokens_to_sell = intent.amount.raw;

    if tokens_to_sell.is_zero() {
        return Ok(failed_report_at(
            order_id,
            "zero token amount; nothing to sell",
            block,
        ));
    }
    let mut chain = match live_simulator.start_chain_at(block).await {
        Ok(chain) => chain,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("live simulator state unavailable for sell block {block}: {error}"),
                block,
            ));
        }
    };
    let params = match pool_simulation_parameters_live(
        &mut chain,
        pool,
        intent.token_address,
        U256::ZERO,
        block,
    ) {
        Ok(params) => params,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("invalid pool parameters for live chain simulation: {error}"),
                block,
            ));
        }
    };
    let denom_decimals = params.denom_decimals;
    let result: SellSwapResult = match simulate_sell_swap_with_params_and_chain(
        simulator,
        tx_processor.clone(),
        params,
        tokens_to_sell,
        chain,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return Ok(failed_report_at(
                order_id,
                format!("live chain sell simulation failed: {error}"),
                block,
            ));
        }
    };
    if !result.success {
        return Ok(failed_report_at_with_gas(
            order_id,
            result
                .failure_reason
                .unwrap_or("sell simulation failed".to_string()),
            block,
            Some(result.gas_used),
            Some(result.gas_cost),
        ));
    }

    let denom_received = Amount {
        raw: result.denom_received,
        decimals: denom_decimals,
    };

    if skip_uneconomic_sell {
        if let Some(reason) =
            uneconomic_sell_cancellation_reason(pool, result.denom_received, result.gas_cost)
        {
            return Ok(cancelled_report_at(order_id, reason, block));
        }
    }

    Ok(ExecutionReport {
        order_id,
        status: ExecutionStatus::Confirmed,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: Some(denom_received),
        token_amount: None,
        gas_used: Some(result.gas_used),
        gas_cost: Some(gas_cost_amount(result.gas_cost)),
        mined_evidence: None,
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

fn pool_simulation_parameters_live(
    chain: &mut UnsignedTxChainSimulation,
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
        None => query_erc20_decimals_on_live_chain(chain, token_address)?,
    };
    let denom_address = pool
        .denom_address
        .ok_or_else(|| format!("pool {} has no denomination address", pool.address))?;
    let denom_decimals = denom_decimals_live(chain, denom_address)?;

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
        PoolProtocol::PancakeSwapV2 => Some(PoolType::PancakeSwapV2),
        PoolProtocol::Unknown(s) => match s.as_str() {
            "sushi" | "sushiswap" => Some(PoolType::SushiSwap),
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

fn denom_decimals_live(
    chain: &mut UnsignedTxChainSimulation,
    denom_address: Address,
) -> std::result::Result<u8, String> {
    if denom_address.is_zero() || denom_address == WETH_ADDRESS || denom_address == DAI_ADDRESS {
        return Ok(18);
    }
    if denom_address == USDC_ADDRESS || denom_address == USDT_ADDRESS {
        return Ok(6);
    }
    query_erc20_decimals_on_live_chain(chain, denom_address)
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

fn query_erc20_decimals_on_live_chain(
    chain: &mut UnsignedTxChainSimulation,
    token_address: Address,
) -> std::result::Result<u8, String> {
    let result = chain
        .simulate_view_call(token_address, Bytes::from_static(&ERC20_DECIMALS_SELECTOR))
        .map_err(|error| format!("token decimals live view simulation failed: {error}"))?;
    if !result.success {
        return Err("token decimals live view simulation reverted".to_string());
    }
    if result.output.len() < 32 {
        return Err(format!(
            "token decimals live view simulation returned short output: {} bytes",
            result.output.len()
        ));
    }
    Ok(result.decode_uint8())
}

fn failed_report(order_id: OrderId, reason: impl Into<String>) -> ExecutionReport {
    failed_report_with_block(order_id, reason, None)
}

fn failed_report_at(order_id: OrderId, reason: impl Into<String>, block: u64) -> ExecutionReport {
    failed_report_with_block(order_id, reason, Some(block))
}

fn cancelled_report_at(
    order_id: OrderId,
    reason: impl Into<String>,
    block: u64,
) -> ExecutionReport {
    ExecutionReport {
        order_id,
        status: ExecutionStatus::Cancelled,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        mined_evidence: None,
        error: Some(reason.into()),
    }
}

fn failed_report_at_with_gas(
    order_id: OrderId,
    reason: impl Into<String>,
    block: u64,
    gas_used: Option<u64>,
    gas_cost: Option<U256>,
) -> ExecutionReport {
    let mut report = failed_report_with_block(order_id, reason, Some(block));
    report.gas_used = gas_used;
    report.gas_cost = gas_cost.map(gas_cost_amount);
    report
}

fn failed_report_with_block(
    order_id: OrderId,
    reason: impl Into<String>,
    block_number: Option<u64>,
) -> ExecutionReport {
    ExecutionReport {
        order_id,
        status: ExecutionStatus::Failed,
        tx_hash: None,
        block_number,
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        mined_evidence: None,
        error: Some(reason.into()),
    }
}

fn submitted_live_chain_sim_report(
    order_id: OrderId,
    submitted_block: u64,
    expected_confirmation_block: u64,
) -> ExecutionReport {
    let mut report = ExecutionReport {
        order_id,
        status: ExecutionStatus::Submitted,
        tx_hash: None,
        block_number: Some(submitted_block),
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        mined_evidence: None,
        error: None,
    };
    let mut evidence = report.mined_evidence.unwrap_or_default();
    evidence.submitted_block_number = Some(submitted_block);
    evidence.expected_confirmation_block = Some(expected_confirmation_block);
    evidence.receipt_status = Some("live_backtest_chain_sim_submitted".to_string());
    report.mined_evidence = Some(evidence);
    report
}

fn with_live_chain_sim_evidence(
    mut report: ExecutionReport,
    decision_block: u64,
    expected_confirmation_block: u64,
    simulation_block: Option<u64>,
) -> ExecutionReport {
    let receipt_block = report.block_number;
    let mut evidence = report.mined_evidence.unwrap_or_default();
    evidence.submitted_block_number = Some(decision_block);
    evidence.expected_confirmation_block = Some(expected_confirmation_block);
    evidence.receipt_block_number = receipt_block;
    evidence.simulation_block_number = simulation_block;
    evidence.confirmation_lag_blocks =
        receipt_block.map(|block| block as i64 - expected_confirmation_block as i64);
    evidence.receipt_status = Some("live_backtest_chain_sim".to_string());
    report.mined_evidence = Some(evidence);
    report
}

fn gas_cost_amount(raw: U256) -> Amount {
    Amount { raw, decimals: 18 }
}

fn sell_intent_for_position(position: &Position) -> Option<OrderIntent> {
    Some(OrderIntent {
        trade_id: Some(position.trade_id.clone()),
        portfolio_id: position.key.portfolio_id.clone(),
        wallet_id: position.key.wallet_id.clone(),
        strategy_name: position.key.strategy_name.clone(),
        side: OrderSide::Sell,
        token_address: position.key.token_address,
        pool_address: position.key.pool_address.clone(),
        protocol: position.key.protocol.clone(),
        amount: position.entry_token_raw_amount.clone()?,
        route: None,
        max_slippage_bps: 0,
        deadline_secs: 0,
        decision_reason: None,
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
            Some(PositionValueSimulation {
                block_number,
                current_value: Amount::zero(18),
                gas_used: report.gas_used,
                error: Some(error),
            })
        }
        ExecutionStatus::Submitted
        | ExecutionStatus::Pending
        | ExecutionStatus::Deferred
        | ExecutionStatus::Cancelled => None,
    }
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

#[cfg(test)]
mod tests {
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
