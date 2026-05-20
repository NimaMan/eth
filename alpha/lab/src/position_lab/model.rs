use serde::{Deserialize, Serialize};

use crate::RunMetadata;

#[derive(Clone, Debug)]
pub enum PositionSelector {
    PositionId(String),
    Token(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionReport {
    pub run: RunMetadata,
    pub position: PositionRecord,
    pub entry_report: Option<ExecutionReportRecord>,
    pub latest_snapshot: Option<SnapshotRecord>,
    pub entry_observation: Option<PoolObservation>,
    pub latest_observation: Option<PoolObservation>,
    pub trajectory: Vec<TrajectoryPoint>,
    pub checks: Vec<PositionCheck>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionRecord {
    pub run_id: String,
    pub position_id: String,
    pub token_address: String,
    pub pool_address: String,
    pub state: String,
    pub entry_order_id: Option<String>,
    pub entry_block: Option<i64>,
    pub entry_cost_eth: Option<String>,
    pub entry_token_amount: Option<String>,
    pub entry_token_raw: Option<String>,
    pub entry_token_decimals: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionReportRecord {
    pub order_id: String,
    pub status: String,
    pub block_number: Option<i64>,
    pub filled_amount_raw: Option<String>,
    pub filled_amount_decimals: Option<i16>,
    pub gas_used: Option<i64>,
    pub error: Option<String>,
    pub token_amount_raw: Option<String>,
    pub token_amount_decimals: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub block_number: Option<i64>,
    pub current_value_eth: Option<String>,
    pub realized_profit_eth: Option<String>,
    pub unrealized_profit_eth: Option<String>,
    pub roi: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolObservation {
    pub block_number: Option<i64>,
    pub protocol: Option<String>,
    pub denom_symbol: Option<String>,
    pub denom_reserve: Option<String>,
    pub token_reserve: Option<String>,
    pub price: Option<String>,
    pub can_buy: Option<bool>,
    pub can_sell: Option<bool>,
    pub is_scam: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrajectoryPoint {
    pub block_number: Option<i64>,
    pub current_value_eth: Option<String>,
    pub unrealized_profit_eth: Option<String>,
    pub roi: Option<String>,
    pub denom_reserve: Option<String>,
    pub token_reserve: Option<String>,
    pub spot_price: Option<String>,
    pub can_buy: Option<bool>,
    pub can_sell: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionCheck {
    pub status: String,
    pub code: String,
    pub message: String,
}
