use alloy_primitives::{Address, Log as AlloyLog, B256, U256};
use eth_alpha_core::{
    execution::ExecutionReport, ids::OrderId, market::PoolSnapshot, order::OrderIntent,
};
use serde::{Deserialize, Serialize};
use tx_processor::{
    PoolBuySellSimulationResult, PoolType, ProcessedTransaction, UniswapV4PoolConfig,
};
use tx_simulator::UnsignedTransaction;

#[derive(Debug, Clone, Deserialize)]
pub struct LiveUnsignedTxSimulationRequest {
    pub block: u64,
    pub transaction: UnsignedTransaction,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveUnsignedTxSimulationResponse {
    pub schema: &'static str,
    pub block: u64,
    pub block_hash: Option<B256>,
    pub state_source: &'static str,
    pub success: bool,
    pub gas_used: u64,
    pub effective_gas_price_wei: Option<String>,
    pub tx_type: Option<u8>,
    pub revert_reason: Option<String>,
    pub log_count: usize,
    pub logs: Vec<AlloyLog>,
    pub base_fee_per_gas_wei: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveTxSimulatorStatusResponse {
    pub schema: &'static str,
    pub available: bool,
    pub unavailable_reason: Option<String>,
    pub selected_block_number: Option<u64>,
    pub selected_block_hash: Option<B256>,
    pub state_source: &'static str,
    pub latest_reth_finished_block_number: Option<u64>,
    pub latest_historical_context_block_number: Option<u64>,
    pub latest_live_block_number: Option<u64>,
    pub latest_tracked_state_block_number: Option<u64>,
    pub base_fee_per_gas_wei: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LiveUnsignedTxSequenceSimulationRequest {
    pub block: u64,
    pub transactions: Vec<UnsignedTransaction>,
    #[serde(default)]
    pub stop_on_revert: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveUnsignedTxSequenceSimulationResponse {
    pub schema: &'static str,
    pub state_available: bool,
    pub unavailable_reason: Option<String>,
    pub block: u64,
    pub block_hash: Option<B256>,
    pub state_source: &'static str,
    pub processed_transactions: Vec<ProcessedTransaction>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LivePoolBuySellSimulationRequest {
    pub block: u64,
    pub config: LivePoolBuySellSimulationConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LivePoolBuySellSimulationConfig {
    pub token_address: Address,
    pub pool_address: Address,
    pub pool_type: PoolType,
    pub test_amount: U256,
    pub buyer_address: Address,
    #[serde(default)]
    pub prior_txs: Vec<ProcessedTransaction>,
    #[serde(default)]
    pub slippage_tolerance: f64,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub buy_gas_limit: u64,
    pub approve_gas_limit: u64,
    pub sell_gas_limit: u64,
    pub weth_address: Address,
    pub denom_address: Address,
    pub denom_decimals: u8,
    #[serde(default)]
    pub block_delay: u64,
    pub token_decimals: u8,
    pub uniswap_v4_config: Option<UniswapV4PoolConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LivePoolBuySellSimulationResponse {
    pub schema: &'static str,
    pub state_available: bool,
    pub unavailable_reason: Option<String>,
    pub block: u64,
    pub block_hash: Option<B256>,
    pub state_source: &'static str,
    pub result: Option<PoolBuySellSimulationResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LiveOrderSimulationRequest {
    pub order_id: OrderId,
    pub intent: OrderIntent,
    pub pool: PoolSnapshot,
    pub submitted_block: u64,
    pub execution_block: u64,
    #[serde(default)]
    pub expected_block_hash: Option<B256>,
    #[serde(default)]
    pub expected_parent_hash: Option<B256>,
    #[serde(default)]
    pub skip_uneconomic_sell: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveOrderSimulationResponse {
    pub schema: &'static str,
    pub state_available: bool,
    pub unavailable_reason: Option<String>,
    pub block: u64,
    pub block_hash: Option<B256>,
    pub parent_block: Option<u64>,
    pub parent_block_hash: Option<B256>,
    pub state_source: &'static str,
    pub report: Option<ExecutionReport>,
}
