use alloy_primitives::U256;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::pnl::common::normalize_address_string;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddressPoolPosition {
    pub address: String,
    pub token_in_raw: U256,
    pub token_out_raw: U256,
    pub denom_in_raw: U256,
    pub denom_out_raw: U256,
    pub native_fee_raw: U256,
    #[serde(default, alias = "native_bribe_raw")]
    pub native_priority_fee_raw: U256,
    pub first_block: Option<u64>,
    pub latest_block: Option<u64>,
    pub movement_count: u64,
}

impl AddressPoolPosition {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: normalize_address_string(address.into()),
            token_in_raw: U256::ZERO,
            token_out_raw: U256::ZERO,
            denom_in_raw: U256::ZERO,
            denom_out_raw: U256::ZERO,
            native_fee_raw: U256::ZERO,
            native_priority_fee_raw: U256::ZERO,
            first_block: None,
            latest_block: None,
            movement_count: 0,
        }
    }

    pub(crate) fn record_token_in(&mut self, amount: U256, block_number: u64) {
        self.token_in_raw = self.token_in_raw.saturating_add(amount);
        self.touch(block_number);
    }

    pub(crate) fn record_token_out(&mut self, amount: U256, block_number: u64) {
        self.token_out_raw = self.token_out_raw.saturating_add(amount);
        self.touch(block_number);
    }

    pub(crate) fn record_denom_in(&mut self, amount: U256, block_number: u64) {
        self.denom_in_raw = self.denom_in_raw.saturating_add(amount);
        self.touch(block_number);
    }

    pub(crate) fn record_denom_out(&mut self, amount: U256, block_number: u64) {
        self.denom_out_raw = self.denom_out_raw.saturating_add(amount);
        self.touch(block_number);
    }

    pub(crate) fn record_native_fee(&mut self, amount: U256, block_number: u64) {
        self.native_fee_raw = self.native_fee_raw.saturating_add(amount);
        self.touch(block_number);
    }

    pub(crate) fn record_native_priority_fee(&mut self, amount: U256, block_number: u64) {
        self.native_priority_fee_raw = self.native_priority_fee_raw.saturating_add(amount);
        self.touch(block_number);
    }

    pub(crate) fn touch(&mut self, block_number: u64) {
        if self.first_block.is_none() {
            self.first_block = Some(block_number);
        }
        self.latest_block = Some(block_number);
        self.movement_count = self.movement_count.saturating_add(1);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddressPoolPnlSummary {
    pub address: String,
    pub token_balance_raw: String,
    pub denom_cashflow_raw: String,
    pub native_fee_raw: String,
    #[serde(alias = "native_bribe_raw")]
    pub native_priority_fee_raw: String,
    pub token_balance: Decimal,
    pub denom_cashflow: Decimal,
    pub native_fee: Decimal,
    #[serde(alias = "native_bribe")]
    pub native_priority_fee: Decimal,
    pub marked_token_value_denom: Option<Decimal>,
    pub pnl_proxy_denom: Option<Decimal>,
    pub token_in_raw: String,
    pub token_out_raw: String,
    pub denom_in_raw: String,
    pub denom_out_raw: String,
    pub first_block: Option<u64>,
    pub latest_block: Option<u64>,
    pub movement_count: u64,
}
