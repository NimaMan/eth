use alloy_primitives::U256;
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

use crate::pnl::common::hash_string;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PoolPnlEntry {
    pub tx_hash: String,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx_index: u64,
    pub log_index: Option<u64>,
    pub address: String,
    pub kind: PoolPnlEntryKind,
    pub token_in_raw: String,
    pub token_out_raw: String,
    pub denom_in_raw: String,
    pub denom_out_raw: String,
    pub native_fee_raw: String,
    #[serde(alias = "native_bribe_raw")]
    pub native_priority_fee_raw: String,
    pub pool_direct: bool,
}

impl PoolPnlEntry {
    pub(crate) fn new(
        transaction: &ProcessedTransaction,
        log_index: Option<u64>,
        address: String,
        kind: PoolPnlEntryKind,
        token_in_raw: U256,
        token_out_raw: U256,
        denom_in_raw: U256,
        denom_out_raw: U256,
        native_fee_raw: U256,
        native_priority_fee_raw: U256,
        pool_direct: bool,
    ) -> Self {
        Self {
            tx_hash: hash_string(&transaction.hash),
            block_number: transaction.block_number,
            block_timestamp: transaction.block_timestamp,
            tx_index: transaction.tx_index,
            log_index,
            address,
            kind,
            token_in_raw: token_in_raw.to_string(),
            token_out_raw: token_out_raw.to_string(),
            denom_in_raw: denom_in_raw.to_string(),
            denom_out_raw: denom_out_raw.to_string(),
            native_fee_raw: native_fee_raw.to_string(),
            native_priority_fee_raw: native_priority_fee_raw.to_string(),
            pool_direct,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolPnlEntryKind {
    TokenIn,
    TokenOut,
    DenomIn,
    DenomOut,
    NativeDenomIn,
    NativeDenomOut,
    NativeFee,
    #[serde(alias = "native_bribe")]
    NativePriorityFee,
}

impl PoolPnlEntryKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::TokenIn => "token_in",
            Self::TokenOut => "token_out",
            Self::DenomIn => "denom_in",
            Self::DenomOut => "denom_out",
            Self::NativeDenomIn => "native_denom_in",
            Self::NativeDenomOut => "native_denom_out",
            Self::NativeFee => "native_fee",
            Self::NativePriorityFee => "native_priority_fee",
        }
    }
}
