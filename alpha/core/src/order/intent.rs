use crate::{
    amount::Amount,
    ids::{PoolAddress, PortfolioId, StrategyName, TokenAddress, TradeId, WalletId},
    order::RouteHint,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrderIntent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_id: Option<TradeId>,
    pub portfolio_id: PortfolioId,
    pub wallet_id: WalletId,
    pub strategy_name: StrategyName,
    pub side: OrderSide,
    pub token_address: TokenAddress,
    pub pool_address: PoolAddress,
    pub amount: Amount,
    pub route: Option<RouteHint>,
    pub max_slippage_bps: u32,
    pub deadline_secs: u64,
}
