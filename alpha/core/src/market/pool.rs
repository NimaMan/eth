use crate::{
    amount::DecimalAmount,
    ids::{BlockNumber, PoolAddress, TokenAddress},
};
use alloy_primitives::B256;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PoolProtocol {
    UniswapV2,
    UniswapV3,
    UniswapV4,
    PancakeSwapV2,
    Unknown(String),
}

impl Default for PoolProtocol {
    fn default() -> Self {
        Self::Unknown("unknown".to_string())
    }
}

impl PoolProtocol {
    pub fn from_label(value: &str) -> Self {
        let trimmed = value.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "uniswapv2" | "uniswap_v2" | "uniswap-v2" | "v2" => Self::UniswapV2,
            "uniswapv3" | "uniswap_v3" | "uniswap-v3" | "v3" => Self::UniswapV3,
            "uniswapv4" | "uniswap_v4" | "uniswap-v4" | "v4" => Self::UniswapV4,
            "pancake" | "pancakeswap" | "pancake_v2" | "pancake-v2" | "pancakeswap_v2"
            | "pancakeswap-v2" => Self::PancakeSwapV2,
            "" => Self::default(),
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn label(&self) -> Cow<'_, str> {
        match self {
            Self::UniswapV2 => Cow::Borrowed("UNISWAP-V2"),
            Self::UniswapV3 => Cow::Borrowed("UNISWAP-V3"),
            Self::UniswapV4 => Cow::Borrowed("UNISWAP-V4"),
            Self::PancakeSwapV2 => Cow::Borrowed("PANCAKESWAP-V2"),
            Self::Unknown(value) if value.trim().is_empty() => Cow::Borrowed("unknown"),
            Self::Unknown(value) => Cow::Borrowed(value.as_str()),
        }
    }
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
