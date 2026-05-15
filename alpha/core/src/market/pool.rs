use crate::{
    amount::DecimalAmount,
    ids::{BlockNumber, PoolAddress, TokenAddress},
};
use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PoolProtocol {
    UniswapV2,
    UniswapV3,
    UniswapV4,
    PancakeSwapV2,
    Unknown(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4PoolKeySnapshot {
    pub pool_manager: TokenAddress,
    pub pool_id: B256,
    pub currency0: TokenAddress,
    pub currency1: TokenAddress,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: TokenAddress,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolSnapshot {
    pub address: PoolAddress,
    pub token_address: TokenAddress,
    pub protocol: PoolProtocol,
    pub denom_address: Option<TokenAddress>,
    pub denom_symbol: Option<String>,
    pub denom_reserve: DecimalAmount,
    pub token_reserve: DecimalAmount,
    pub price_denom_per_token: Option<DecimalAmount>,
    pub initial_price_denom_per_token: Option<DecimalAmount>,
    pub price_ratio_to_initial: Option<DecimalAmount>,
    pub token_decimals: Option<u8>,
    pub fee_tier: Option<u32>,
    pub uniswap_v4: Option<UniswapV4PoolKeySnapshot>,
    pub latest_block: BlockNumber,
    pub can_buy: bool,
    pub can_sell: bool,
    pub is_scam: bool,
}
