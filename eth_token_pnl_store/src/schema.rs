use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PnlCalculationRun {
    pub run_id: String,
    pub chain_id: i64,
    pub mode: String,
    pub algorithm_version: String,
    pub start_block: Option<i64>,
    pub end_block: Option<i64>,
    pub include_traces: bool,
    pub status: String,
    pub metadata: Value,
}

impl PnlCalculationRun {
    pub fn historical(
        run_id: impl Into<String>,
        algorithm_version: impl Into<String>,
        start_block: u64,
        end_block: u64,
        include_traces: bool,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            chain_id: 1,
            mode: "historical".to_string(),
            algorithm_version: algorithm_version.into(),
            start_block: Some(start_block as i64),
            end_block: Some(end_block as i64),
            include_traces,
            status: "running".to_string(),
            metadata: Value::Object(Default::default()),
        }
    }
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PoolPnlStateRow {
    pub run_id: String,
    pub pool_id: String,
    pub token_address: String,
    pub denom_address: String,
    pub protocol: Option<String>,
    pub token_decimals: i16,
    pub denom_decimals: i16,
    pub tx_count: i64,
    pub latest_block: Option<i64>,
    pub latest_timestamp: Option<i64>,
    pub token_in_raw: String,
    pub token_out_raw: String,
    pub denom_in_raw: String,
    pub denom_out_raw: String,
    pub pool_token_in_raw: String,
    pub pool_token_out_raw: String,
    pub pool_denom_in_raw: String,
    pub pool_denom_out_raw: String,
    pub native_fee_raw: String,
    pub native_bribe_raw: String,
    pub token_transfer_count: i64,
    pub denom_transfer_count: i64,
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
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct AddressPnlRow {
    pub run_id: String,
    pub pool_id: String,
    pub address: String,
    pub token_in_raw: String,
    pub token_out_raw: String,
    pub denom_in_raw: String,
    pub denom_out_raw: String,
    pub native_fee_raw: String,
    pub native_bribe_raw: String,
    pub token_balance_raw: String,
    pub denom_cashflow_raw: String,
    pub token_balance: Option<Decimal>,
    pub denom_cashflow: Option<Decimal>,
    pub native_fee: Option<Decimal>,
    pub native_bribe: Option<Decimal>,
    pub marked_token_value_denom: Option<Decimal>,
    pub pnl_proxy_denom: Option<Decimal>,
    pub first_block: Option<i64>,
    pub latest_block: Option<i64>,
    pub movement_count: i64,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PnlMovementRow {
    pub run_id: String,
    pub pool_id: String,
    pub entry_index: i64,
    pub tx_hash: String,
    pub block_number: i64,
    pub block_timestamp: i64,
    pub tx_index: i64,
    pub log_index: Option<i64>,
    pub address: String,
    pub kind: String,
    pub token_in_raw: String,
    pub token_out_raw: String,
    pub denom_in_raw: String,
    pub denom_out_raw: String,
    pub native_fee_raw: String,
    pub native_bribe_raw: String,
    pub pool_direct: bool,
}
