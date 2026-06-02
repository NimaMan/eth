use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Pool and token state captured at export time.
/// Filled by the caller who holds both ERC20Token and BasePool.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PnlPoolMeta {
    pub token_creator_address: Option<String>,
    pub pool_creator_address: Option<String>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub lifecycle: Option<String>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub scam_mechanism: Option<String>,
    pub eligible: bool,
    pub eligible_outcome: Option<String>,
    #[serde(default)]
    pub pool_labels: Vec<String>,
    #[serde(default)]
    pub pool_state_flags: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PnlPoolExport {
    pub pool_id: String,
    pub token_address: String,
    pub denom_address: String,
    pub protocol: Option<String>,
    pub token_decimals: u8,
    pub denom_decimals: u8,
    pub tx_count: u64,
    pub latest_block_number: Option<u64>,
    pub latest_block_timestamp: Option<u64>,
    pub conservation: PnlConservationExport,
    pub address_positions: Vec<PnlAddressPositionExport>,
    pub movements: Vec<PnlMovementExport>,
    #[serde(default)]
    pub meta: PnlPoolMeta,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PnlConservationExport {
    pub token_in_raw: String,
    pub token_out_raw: String,
    pub denom_in_raw: String,
    pub denom_out_raw: String,
    pub pool_token_in_raw: String,
    pub pool_token_out_raw: String,
    pub pool_denom_in_raw: String,
    pub pool_denom_out_raw: String,
    pub native_fee_raw: String,
    #[serde(alias = "native_bribe_raw")]
    pub native_priority_fee_raw: String,
    pub token_transfer_count: u64,
    pub denom_transfer_count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PnlAddressPositionExport {
    pub address: String,
    pub token_in_raw: String,
    pub token_out_raw: String,
    pub denom_in_raw: String,
    pub denom_out_raw: String,
    pub native_fee_raw: String,
    #[serde(alias = "native_bribe_raw")]
    pub native_priority_fee_raw: String,
    pub first_block: Option<u64>,
    pub latest_block: Option<u64>,
    pub movement_count: u64,
    pub token_balance_raw: String,
    pub denom_cashflow_raw: String,
    pub token_balance: Decimal,
    pub denom_cashflow: Decimal,
    pub native_fee: Decimal,
    #[serde(alias = "native_bribe")]
    pub native_priority_fee: Decimal,
    pub marked_token_value_denom: Option<Decimal>,
    pub pnl_proxy_denom: Option<Decimal>,
    #[serde(default = "default_position_status")]
    pub position_status: String,
    #[serde(default = "default_valuation_status")]
    pub valuation_status: String,
    #[serde(default = "default_reconciliation_status")]
    pub reconciliation_status: String,
    #[serde(default)]
    pub realized_pnl_denom: Option<Decimal>,
    #[serde(default)]
    pub unrealized_value_denom: Option<Decimal>,
    #[serde(default)]
    pub total_pnl_denom: Option<Decimal>,
    #[serde(default)]
    pub movement_rows_retained: u64,
    #[serde(default)]
    pub movement_rows_backed: bool,
    #[serde(default)]
    pub actor_roles: Vec<String>,
    #[serde(default)]
    pub is_user_candidate: bool,
    #[serde(default = "default_accounting_context")]
    pub accounting_context: Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PnlMovementExport {
    pub entry_index: u64,
    pub tx_hash: String,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx_index: u64,
    pub log_index: Option<u64>,
    pub address: String,
    pub kind: String,
    pub token_in_raw: String,
    pub token_out_raw: String,
    pub denom_in_raw: String,
    pub denom_out_raw: String,
    pub native_fee_raw: String,
    #[serde(alias = "native_bribe_raw")]
    pub native_priority_fee_raw: String,
    pub pool_direct: bool,
}

fn default_position_status() -> String {
    "unknown".to_string()
}

fn default_valuation_status() -> String {
    "unknown".to_string()
}

fn default_reconciliation_status() -> String {
    "unknown".to_string()
}

fn default_accounting_context() -> Value {
    Value::Object(Default::default())
}
