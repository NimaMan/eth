use eth_alpha_core::{
    ids::{OrderId, PositionId, TokenAddress, TradeId, TxHash},
    order::{OrderIntent, OrderSide},
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
    pub submitted_block_number: Option<u64>,
    pub position_id: PositionId,
    pub trade_id: Option<TradeId>,
    pub order_side: OrderSide,
    pub token_address: TokenAddress,
    pub selected_gas_limit: Option<String>,
    pub selected_max_fee_per_gas_wei: Option<String>,
    pub selected_max_priority_fee_per_gas_wei: Option<String>,
    pub selected_bribe_priority_fee_per_gas_wei: Option<String>,
    pub selected_bribe_max_fee_per_gas_wei: Option<String>,
    pub gas_policy_action: Option<String>,
    pub gas_policy_signal: Option<String>,
    pub gas_policy_status: Option<String>,
    pub gas_policy_profile: Option<String>,
    pub gas_policy_profiles: Option<Vec<String>>,
    pub gas_rank_source: Option<String>,
    pub gas_estimated_max_cost_eth: Option<String>,
    pub gas_estimated_priority_spend_eth: Option<String>,
    pub gas_policy_guard: Option<String>,
    pub private_execution_transport: Option<String>,
    pub bundle_hash: Option<String>,
    pub bundle_target_block: Option<u64>,
    pub bundle_max_block: Option<u64>,
    pub gas_policy_tail_after_tx_hash: Option<String>,
    pub gas_policy_dependency_priority_fee_wei: Option<String>,
    pub gas_policy_dependency_gas_price_wei: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChainSimSubmittedExecutionRecord {
    pub order_id: OrderId,
    pub submitted_block_number: Option<u64>,
    pub expected_confirmation_block: Option<u64>,
    pub position_id: PositionId,
    pub trade_id: Option<TradeId>,
    pub order_side: OrderSide,
    pub token_address: TokenAddress,
    pub intent: OrderIntent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManualCloseRequest {
    pub request_id: String,
    pub run_id: String,
    pub strategy_name: String,
    pub trade_id: String,
    pub position_id: Option<String>,
    pub token_address: String,
    pub pool_address: String,
    pub requested_percent: Option<String>,
    pub requested_raw_amount: Option<String>,
    pub reason_code: String,
    pub payload: Value,
}
