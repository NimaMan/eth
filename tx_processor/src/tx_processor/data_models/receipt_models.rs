use alloy_primitives::{Address, B256, U256};
use reth_chain_query::utils::checksum::{deserialize_address_checksum, serialize_address_checksum};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::serde_helpers::{
    deserialize_i128_from_any, deserialize_u128_from_any, serialize_i128_to_string,
    serialize_u128_to_string,
};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ERC20TransferEvent {
    #[serde(
        serialize_with = "serialize_address_checksum",
        deserialize_with = "deserialize_address_checksum"
    )]
    pub token_address: Address,
    #[serde(
        serialize_with = "serialize_address_checksum",
        deserialize_with = "deserialize_address_checksum"
    )]
    pub from_address: Address,
    #[serde(
        serialize_with = "serialize_address_checksum",
        deserialize_with = "deserialize_address_checksum"
    )]
    pub to_address: Address,
    pub amount: U256,
    pub log_index: u64,
}

impl fmt::Debug for ERC20TransferEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ERC20TransferEvent")
            .field(
                "token_address",
                &reth_chain_query::to_checksum_address(&self.token_address),
            )
            .field(
                "from_address",
                &reth_chain_query::to_checksum_address(&self.from_address),
            )
            .field(
                "to_address",
                &reth_chain_query::to_checksum_address(&self.to_address),
            )
            .field("amount", &self.amount)
            .field("log_index", &self.log_index)
            .finish()
    }
}

impl fmt::Display for ERC20TransferEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Transfer {} tokens from {} to {}",
            self.amount,
            reth_chain_query::to_checksum_address(&self.from_address),
            reth_chain_query::to_checksum_address(&self.to_address)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ERC721TransferEvent {
    #[serde(
        serialize_with = "serialize_address_checksum",
        deserialize_with = "deserialize_address_checksum"
    )]
    pub token_address: Address,
    #[serde(
        serialize_with = "serialize_address_checksum",
        deserialize_with = "deserialize_address_checksum"
    )]
    pub from_address: Address,
    #[serde(
        serialize_with = "serialize_address_checksum",
        deserialize_with = "deserialize_address_checksum"
    )]
    pub to_address: Address,
    pub token_id: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ERC1155TransferEvent {
    pub token_address: Address,
    pub operator: Address,
    pub from_address: Address,
    pub to_address: Address,
    pub token_ids: Vec<U256>,
    pub amounts: Vec<U256>,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2SyncEvent {
    pub pair_address: Address,
    pub reserve0: U256,
    pub reserve1: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2SwapEvent {
    pub pair_address: Address,
    pub sender: Address,
    pub to: Address,
    #[serde(rename = "amount0In")]
    pub amount0_in: U256,
    #[serde(rename = "amount1In")]
    pub amount1_in: U256,
    #[serde(rename = "amount0Out")]
    pub amount0_out: U256,
    #[serde(rename = "amount1Out")]
    pub amount1_out: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ERC20ApprovalEvent {
    pub token_address: Address,
    pub owner: Address,
    pub spender: Address,
    pub amount: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ERC721ApprovalEvent {
    pub token_address: Address,
    pub owner: Address,
    pub approved_address: Address,
    pub token_id: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalForAllEvent {
    pub token_address: Address,
    pub owner: Address,
    pub operator: Address,
    pub approved: bool,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2PairCreatedEvent {
    pub pair_address: Address,
    pub token0: Address,
    pub token1: Address,
    #[serde(default)]
    pub factory_address: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DepositEvent {
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
pub struct WithdrawEvent {
    pub pair_address: Address,
    pub sender: Option<Address>,
    pub amount: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2MintEvent {
    pub pair_address: Address,
    pub sender: Address,
    pub amount0: U256,
    pub amount1: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV2BurnEvent {
    pub pair_address: Address,
    pub sender: Address,
    pub amount0: U256,
    pub amount1: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DexSwapEvent {
    pub dex_name: String,
    pub token_in: ERC20TransferEvent,
    pub token_out: ERC20TransferEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnershipTransferredEvent {
    pub contract_address: Address,
    pub previous_owner: Address,
    pub new_owner: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnershipTransferStartedEvent {
    pub contract_address: Address,
    pub previous_owner: Address,
    pub new_owner: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessControlRoleGrantedEvent {
    pub contract_address: Address,
    pub role: B256,
    pub account: Address,
    pub sender: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessControlRoleRevokedEvent {
    pub contract_address: Address,
    pub role: B256,
    pub account: Address,
    pub sender: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProxyAdminChangedEvent {
    pub contract_address: Address,
    pub previous_admin: Address,
    pub new_admin: Address,
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
pub struct UniswapV3PoolCreatedEvent {
    pub factory_address: Address,
    pub token0: Address,
    pub token1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub pool: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3InitializeEvent {
    pub pool_address: Address,
    pub sqrt_price_x96: U256,
    pub tick: i32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3MintEvent {
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
pub struct UniswapV3PositionEvent {
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
pub struct UniswapV3SwapEvent {
    pub pool_address: Address,
    pub sender: Address,
    pub recipient: Address,
    #[serde(
        serialize_with = "serialize_i128_to_string",
        deserialize_with = "deserialize_i128_from_any"
    )]
    pub amount0: i128,
    #[serde(
        serialize_with = "serialize_i128_to_string",
        deserialize_with = "deserialize_i128_from_any"
    )]
    pub amount1: i128,
    pub sqrt_price_x96: U256,
    #[serde(
        serialize_with = "serialize_u128_to_string",
        deserialize_with = "deserialize_u128_from_any"
    )]
    pub liquidity: u128,
    pub tick: i32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3BurnEvent {
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
pub struct UniswapV3DecreaseLiquidityEvent {
    pub token_id: U256,
    pub liquidity: U256,
    pub amount0: U256,
    pub amount1: U256,
    pub pool_address: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3IncreaseLiquidityEvent {
    pub token_id: U256,
    pub liquidity: U256,
    pub amount0: U256,
    pub amount1: U256,
    pub pool_address: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV3CollectEvent {
    pub token_id: U256,
    pub recipient: Address,
    pub amount0: U256,
    pub amount1: U256,
    pub pool_address: Address,
    pub log_index: u64,
}

// Uniswap V4 Events
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4InitializeEvent {
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
pub struct UniswapV4ModifyLiquidityEvent {
    pub pool_manager_address: Address,
    pub event_id: B256,
    pub sender: Address,
    pub tick_lower: i32,
    pub tick_upper: i32,
    #[serde(
        serialize_with = "serialize_i128_to_string",
        deserialize_with = "deserialize_i128_from_any"
    )]
    pub liquidity_delta: i128,
    pub salt: B256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Permit2Event {
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
pub struct UniswapV4SwapEvent {
    pub pool_manager_address: Address,
    pub event_id: B256,
    pub sender: Address,
    #[serde(
        serialize_with = "serialize_i128_to_string",
        deserialize_with = "deserialize_i128_from_any"
    )]
    pub amount0: i128,
    #[serde(
        serialize_with = "serialize_i128_to_string",
        deserialize_with = "deserialize_i128_from_any"
    )]
    pub amount1: i128,
    pub sqrt_price_x96: U256,
    #[serde(
        serialize_with = "serialize_u128_to_string",
        deserialize_with = "deserialize_u128_from_any"
    )]
    pub liquidity: u128,
    pub tick: i32,
    pub fee: u32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4DonateEvent {
    pub pool_manager_address: Address,
    pub event_id: B256,
    pub sender: Address,
    pub amount0: U256,
    pub amount1: U256,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4FeeUpdatedEvent {
    pub pool_manager_address: Address,
    pub event_id: B256,
    pub protocol_fee: u32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4DynamicLPFeeUpdatedEvent {
    pub pool_manager_address: Address,
    pub event_id: B256,
    pub dynamic_lp_fee: u32,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4FeeControllerUpdatedEvent {
    pub pool_manager_address: Address,
    pub protocol_fee_controller: Address,
    pub log_index: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4BalanceDeltaEvent {
    pub pool_manager_address: Address,
    pub pool_id: B256,
    pub settler: Address,
    #[serde(
        serialize_with = "serialize_i128_to_string",
        deserialize_with = "deserialize_i128_from_any"
    )]
    pub delta0: i128,
    #[serde(
        serialize_with = "serialize_i128_to_string",
        deserialize_with = "deserialize_i128_from_any"
    )]
    pub delta1: i128,
    pub log_index: u64,
}
