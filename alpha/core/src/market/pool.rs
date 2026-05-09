use crate::{
    amount::DecimalAmount,
    ids::{BlockNumber, PoolAddress, TokenAddress},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PoolProtocol {
    UniswapV2,
    UniswapV3,
    UniswapV4,
    Unknown(String),
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
    pub latest_block: BlockNumber,
    pub can_buy: bool,
    pub can_sell: bool,
    pub is_scam: bool,
}
