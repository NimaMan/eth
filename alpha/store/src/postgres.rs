use eth_alpha_core::{
    ids::{OrderId, PositionId, TokenAddress, TradeId, TxHash},
    order::OrderSide,
};
use serde_json::Value;
use sqlx::postgres::PgPool;

pub(crate) const DEFAULT_MAX_CONNECTIONS: u32 = 5;

#[derive(Clone)]
pub struct PostgresTradingStore {
    pub(crate) pool: PgPool,
    pub(crate) run_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrategyObservationCursor {
    pub event_source: String,
    pub event_key: String,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub block_number: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct StrategyObservationRecord {
    pub strategy_name: String,
    pub event_source: String,
    pub event_key: String,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub block_number: Option<u64>,
    pub event_timestamp: Option<String>,
    pub decision: String,
    pub report_count: usize,
    pub payload: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveHoldCounterRecord {
    pub position_id: PositionId,
    pub count: u64,
    pub last_block: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmittedExecutionRecord {
    pub order_id: OrderId,
    pub tx_hash: TxHash,
    pub position_id: PositionId,
    pub trade_id: Option<TradeId>,
    pub order_side: OrderSide,
    pub token_address: TokenAddress,
}
