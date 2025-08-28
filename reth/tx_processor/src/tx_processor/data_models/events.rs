use alloy_primitives::{Address, B256, U256};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ERC20Transfer {
    pub token_address: Address,
    pub from_address: Address,
    pub to_address: Address,
    pub amount: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ERC721Transfer {
    pub token_address: Address,
    pub from_address: Address,
    pub to_address: Address,
    pub token_id: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ERC1155Transfer {
    pub token_address: Address,
    pub operator: Address,
    pub from_address: Address,
    pub to_address: Address,
    pub token_ids: Vec<U256>,
    pub amounts: Vec<U256>,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2Sync {
    pub pair_address: Address,
    pub reserve0: U256,
    pub reserve1: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2Swap {
    pub pair_address: Address,
    pub sender: Address,
    pub to: Address,
    pub amount0_in: U256,
    pub amount1_in: U256,
    pub amount0_out: U256,
    pub amount1_out: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ERC20Approval {
    pub token_address: Address,
    pub owner: Address,
    pub spender: Address,
    pub amount: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ERC721Approval {
    pub token_address: Address,
    pub owner: Address,
    pub approved_address: Address,
    pub token_id: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairAction {
    pub pair_address: Address,
    pub token0: Address,
    pub token1: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepositAction {
    pub id: Option<u64>,
    pub token_address: Option<Address>,
    pub withdrawal_address: Option<Address>,
    pub amount: Option<U256>,
    pub unlock_time: Option<u64>,
    pub pair_address: Option<Address>,
    pub sender: Option<Address>,
    pub log_index: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithdrawAction {
    pub pair_address: Address,
    pub sender: Address,
    pub amount: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MintAction {
    pub pair_address: Address,
    pub sender: Address,
    pub amount0: U256,
    pub amount1: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BurnAction {
    pub pair_address: Address,
    pub sender: Address,
    pub amount: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwapAction {
    pub dex_name: String,
    pub token_in: ERC20Transfer,
    pub token_out: ERC20Transfer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnerEvent {
    pub contract_address: Address,
    pub previous_owner: Address,
    pub new_owner: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradingEnabledEvent {
    pub token_address: Address,
    pub block_number: u64,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradingDisabledEvent {
    pub token_address: Address,
    pub block_number: u64,
    pub log_index: u64,
}

// Uniswap V3 Events
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3PoolCreated {
    pub token0: Address,
    pub token1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub pool: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3Initialize {
    pub pool_address: Address,
    pub sqrt_price_x96: U256,
    pub tick: i32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3Mint {
    pub pool_address: Address,
    pub sender: Address,
    pub owner: Address,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount: U256,
    pub amount0: U256,
    pub amount1: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3Position {
    pub token_id: U256,
    pub liquidity: U256,
    pub amount0: U256,
    pub amount1: U256,
    pub pool_address: Address,
    pub owner: Address,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3Swap {
    pub pool_address: Address,
    pub sender: Address,
    pub recipient: Address,
    pub amount0: i128,
    pub amount1: i128,
    pub sqrt_price_x96: U256,
    pub liquidity: u128,
    pub tick: i32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3Burn {
    pub pool_address: Address,
    pub owner: Address,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount: U256,
    pub amount0: U256,
    pub amount1: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3DecreaseLiquidity {
    pub token_id: U256,
    pub liquidity: U256,
    pub amount0: U256,
    pub amount1: U256,
    pub pool_address: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3IncreaseLiquidity {
    pub token_id: U256,
    pub liquidity: U256,
    pub amount0: U256,
    pub amount1: U256,
    pub pool_address: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3Collect {
    pub token_id: U256,
    pub recipient: Address,
    pub amount0: U256,
    pub amount1: U256,
    pub pool_address: Address,
    pub log_index: u64,
}

// Uniswap V4 Events
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4Initialize {
    pub pool_manager_address: Address,
    pub event_id: B256,
    pub currency0: Address,
    pub currency1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
    pub sqrt_price_x96: U256,
    pub tick: i32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4ModifyLiquidity {
    pub pool_manager_address: Address,
    pub event_id: B256,
    pub sender: Address,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity_delta: i128,
    pub salt: B256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Permit2 {
    pub pool_manager_address: Address,
    pub owner: Address,
    pub token: Address,
    pub spender: Address,
    pub amount: U256,
    pub expiration: u64,
    pub nonce: u64,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4Swap {
    pub pool_manager_address: Address,
    pub event_id: B256,
    pub sender: Address,
    pub amount0: i128,
    pub amount1: i128,
    pub sqrt_price_x96: U256,
    pub liquidity: u128,
    pub tick: i32,
    pub fee: u32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4Donate {
    pub pool_manager_address: Address,
    pub event_id: B256,
    pub sender: Address,
    pub amount0: U256,
    pub amount1: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4ProtocolFeeUpdated {
    pub pool_manager_address: Address,
    pub event_id: B256,
    pub protocol_fee: u32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4DynamicLPFeeUpdated {
    pub pool_manager_address: Address,
    pub event_id: B256,
    pub dynamic_lp_fee: u32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4ProtocolFeeControllerUpdated {
    pub pool_manager_address: Address,
    pub protocol_fee_controller: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4BalanceDelta {
    pub pool_manager_address: Address,
    pub pool_id: B256,
    pub settler: Address,
    pub delta0: i128,
    pub delta1: i128,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalTransaction {
    pub from_address: Address,
    pub to_address: Address,
    pub value: U256,
    pub gas_used: u64,
    pub trace_type: String,
    pub call_type: Option<String>,
    pub depth: u32,
    pub error: Option<String>,
}