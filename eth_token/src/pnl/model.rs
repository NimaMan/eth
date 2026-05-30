use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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
    pub native_bribe_raw: String,
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
    pub native_bribe_raw: String,
    pub first_block: Option<u64>,
    pub latest_block: Option<u64>,
    pub movement_count: u64,
    pub token_balance_raw: String,
    pub denom_cashflow_raw: String,
    pub token_balance: Decimal,
    pub denom_cashflow: Decimal,
    pub native_fee: Decimal,
    pub native_bribe: Decimal,
    pub marked_token_value_denom: Option<Decimal>,
    pub pnl_proxy_denom: Option<Decimal>,
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
    pub native_bribe_raw: String,
    pub pool_direct: bool,
}
