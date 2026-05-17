use crate::{
    amount::DecimalAmount,
    ids::{BlockNumber, PositionId, TradeId},
    position::PositionState,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PositionSnapshot {
    pub position_id: PositionId,
    #[serde(default = "legacy_trade_id")]
    pub trade_id: TradeId,
    pub state: PositionState,
    pub block_number: BlockNumber,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_block_number: Option<BlockNumber>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valuation_block_number: Option<BlockNumber>,
    pub current_value_eth: DecimalAmount,
    pub realized_profit_eth: DecimalAmount,
    pub unrealized_profit_eth: DecimalAmount,
    pub roi: DecimalAmount,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_price_to_initial_price_ratio: Option<DecimalAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_initial_price_denom_per_token: Option<DecimalAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_price_denom_per_token: Option<DecimalAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_liquidity_denom: Option<DecimalAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_token_reserve: Option<DecimalAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_denom_symbol: Option<String>,
}

fn legacy_trade_id() -> TradeId {
    TradeId("legacy-unset".to_string())
}
