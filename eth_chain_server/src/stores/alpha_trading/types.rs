use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AlphaTradingStore {
    pub(super) pool: PgPool,
}

#[derive(Debug, Serialize)]
pub struct AlphaStrategyListResponse {
    pub count: usize,
    pub strategies: Vec<AlphaStrategySummary>,
}

#[derive(Debug, Serialize)]
pub struct AlphaStrategyDetailResponse {
    pub strategy: AlphaStrategySummary,
    pub recent_runs: Vec<TraderRunView>,
    pub positions: Vec<PositionView>,
    pub orders: Vec<OrderIntentView>,
    pub execution_reports: Vec<ExecutionReportView>,
    pub risk_events: Vec<RiskEventView>,
    pub strategy_decisions: Vec<StrategyDecisionView>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlphaStrategyResetScope {
    Positions,
    RunState,
}

impl Default for AlphaStrategyResetScope {
    fn default() -> Self {
        Self::Positions
    }
}

impl AlphaStrategyResetScope {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Positions => "positions",
            Self::RunState => "run_state",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AlphaStrategyResetRequest {
    #[serde(default)]
    pub scope: AlphaStrategyResetScope,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResultSetReportQuery {
    pub strategy_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AlphaStrategyResetResponse {
    pub strategy_id: String,
    pub run_id: Option<String>,
    pub scope: AlphaStrategyResetScope,
    pub positions_deleted: u64,
    pub position_snapshots_deleted: u64,
    pub order_intents_deleted: u64,
    pub execution_reports_deleted: u64,
    pub risk_events_deleted: u64,
    pub strategy_decisions_deleted: u64,
    pub strategy_observations_deleted: u64,
    pub requires_trader_restart: bool,
}

#[derive(Debug, Serialize)]
pub struct AlphaStrategySummary {
    pub strategy_id: String,
    pub name: String,
    pub description: String,
    pub mode: Option<String>,
    pub status: String,
    pub run_id: Option<String>,
    pub started_at: Option<String>,
    pub last_heartbeat_at: Option<String>,
    pub stopped_at: Option<String>,
    pub trading_enabled: Option<bool>,
    pub live_status: Option<String>,
    pub live_current_block: Option<u64>,
    pub positions: i64,
    pub open_positions: i64,
    pub orders: i64,
    pub execution_reports: i64,
    pub risk_events: i64,
    pub strategy_decisions: i64,
    pub latest_order_at: Option<String>,
    pub latest_execution_report_at: Option<String>,
    pub latest_risk_event_at: Option<String>,
    pub latest_strategy_decision_at: Option<String>,
    pub rules: Vec<StrategyRuleView>,
}

#[derive(Debug, Serialize)]
pub struct StrategyRuleView {
    pub rule_id: &'static str,
    pub name: &'static str,
    pub status: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Serialize)]
pub struct TraderRunView {
    pub run_id: String,
    pub mode: String,
    pub status: String,
    pub started_at: String,
    pub last_heartbeat_at: String,
    pub stopped_at: Option<String>,
    pub config: Value,
    pub metadata: Value,
}

#[derive(Debug, Serialize)]
pub struct PositionView {
    pub position_id: String,
    pub portfolio_id: String,
    pub wallet_id: String,
    pub strategy_name: String,
    pub token_address: String,
    pub pool_address: String,
    pub protocol: Option<String>,
    pub state: String,
    pub entry_order_id: Option<String>,
    pub exit_order_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct OrderIntentView {
    pub id: i64,
    pub portfolio_id: String,
    pub wallet_id: String,
    pub strategy_name: String,
    pub side: String,
    pub token_address: String,
    pub pool_address: String,
    pub protocol: Option<String>,
    pub amount_raw: String,
    pub amount_decimals: i16,
    pub max_slippage_bps: i32,
    pub deadline_secs: i64,
    pub reason_code: Option<String>,
    pub reason_category: Option<String>,
    pub reason_label: Option<String>,
    pub reason_source: Option<String>,
    pub reason_details: Value,
    pub created_at: String,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct ExecutionReportView {
    pub id: i64,
    pub position_id: Option<String>,
    pub order_side: Option<String>,
    pub order_id: String,
    pub status: String,
    pub tx_hash: Option<String>,
    pub block_number: Option<i64>,
    pub filled_amount_raw: Option<String>,
    pub filled_amount_decimals: Option<i16>,
    pub gas_used: Option<i64>,
    pub error: Option<String>,
    pub created_at: String,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct RiskEventView {
    pub id: i64,
    pub kind: String,
    pub severity: String,
    pub token_address: String,
    pub pool_address: Option<String>,
    pub pending_tx_hash: Option<String>,
    pub observed_block: Option<i64>,
    pub message: String,
    pub created_at: String,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct StrategyDecisionView {
    pub id: i64,
    pub strategy_name: String,
    pub event_source: String,
    pub event_key: String,
    pub block_number: Option<i64>,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub action: String,
    pub reason: Option<String>,
    pub reason_code: Option<String>,
    pub reason_category: Option<String>,
    pub reason_label: Option<String>,
    pub reason_source: Option<String>,
    pub reason_details: Value,
    pub order_side: Option<String>,
    pub created_at: String,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct PositionDecisionAuditView {
    pub bucket: String,
    pub rank: i64,
    pub position_id: String,
    pub token_address: String,
    pub pool_address: String,
    pub state: String,
    pub entry_block: Option<i64>,
    pub snapshot_block: Option<i64>,
    pub pnl_eth: Option<String>,
    pub roi: Option<String>,
    pub entry_reason: Option<String>,
    pub entry_decision_block: Option<i64>,
    pub exit_reason: Option<String>,
    pub exit_decision_block: Option<i64>,
    pub exit_decision_source: Option<String>,
    pub sell_status: Option<String>,
    pub sell_report_block: Option<i64>,
    pub sell_error: Option<String>,
}

#[derive(Default)]
pub(super) struct StrategyCounts {
    pub(super) positions: i64,
    pub(super) open_positions: i64,
    pub(super) orders: i64,
    pub(super) execution_reports: i64,
    pub(super) risk_events: i64,
    pub(super) strategy_decisions: i64,
    pub(super) latest_order_at: Option<String>,
    pub(super) latest_execution_report_at: Option<String>,
    pub(super) latest_risk_event_at: Option<String>,
    pub(super) latest_strategy_decision_at: Option<String>,
}
