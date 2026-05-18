use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeEventTrace {
    pub result_set_id: String,
    pub run_id: String,
    pub strategy_name: String,
    pub trade: TradeRecord,
    pub timing: TradeTiming,
    pub exit_reason: Option<String>,
    pub signal_candidates: Vec<SignalCandidate>,
    pub events: Vec<EventPoint>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeRecord {
    pub trade_id: String,
    pub token_address: String,
    pub pool_address: String,
    pub state: String,
    pub entry_block: Option<i64>,
    pub exit_block: Option<i64>,
    pub latest_snapshot_block: Option<i64>,
    pub entry_cost_eth: Option<String>,
    pub exit_value_eth: Option<String>,
    pub current_value_eth: Option<String>,
    pub gas_cost_eth: Option<String>,
    pub realized_pnl_eth: Option<String>,
    pub unrealized_pnl_eth: Option<String>,
    pub total_pnl_eth: Option<String>,
    pub roi: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TradeTiming {
    pub buy_submitted_block: Option<i64>,
    pub buy_confirmed_block: Option<i64>,
    pub sell_submitted_block: Option<i64>,
    pub sell_confirmed_block: Option<i64>,
    pub first_lp_approval_block: Option<i64>,
    pub first_liquidity_removal_block: Option<i64>,
    pub lp_approval_count: i64,
    pub liquidity_removal_count: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventPoint {
    pub block_number: Option<i64>,
    pub source: String,
    pub kind: String,
    pub label: String,
    pub detail: String,
    pub value_eth: Option<String>,
    pub pnl_eth: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeEventRow {
    pub event_type: String,
    pub order_side: String,
    pub status: String,
    pub order_id: String,
    pub block_number: Option<i64>,
    pub filled_amount_raw: Option<String>,
    pub filled_amount_decimals: Option<i16>,
    pub gas_cost_eth: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskEventRow {
    pub kind: String,
    pub severity: String,
    pub observed_block: Option<i64>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionRow {
    pub event_source: String,
    pub event_key: String,
    pub block_number: Option<i64>,
    pub action: String,
    pub reason: Option<String>,
    pub order_side: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotRow {
    pub block_number: i64,
    pub observed_block_number: Option<i64>,
    pub valuation_block_number: Option<i64>,
    pub state: String,
    pub current_value_eth: String,
    pub realized_pnl_eth: String,
    pub unrealized_pnl_eth: String,
    pub total_pnl_eth: String,
    pub roi: String,
    pub pool_price_to_initial_price_ratio: Option<String>,
    pub pool_liquidity_denom: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignalCandidate {
    pub code: String,
    pub severity: String,
    pub block_number: Option<i64>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LossScanReport {
    pub result_set_id: String,
    pub strategy_name: String,
    pub run_id: Option<String>,
    pub losing_trades: i64,
    pub total_loss_eth: String,
    pub signal_buckets: Vec<SignalBucket>,
    pub exit_reason_buckets: Vec<ExitReasonBucket>,
    pub sample_traces: Vec<TradeEventTrace>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignalBucket {
    pub code: String,
    pub severity: String,
    pub trades: i64,
    pub total_pnl_eth: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExitReasonBucket {
    pub reason: String,
    pub trades: i64,
    pub total_pnl_eth: String,
}
