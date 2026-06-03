use alloy_primitives::U256;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::pnl::common::{scaled_signed_balance, scaled_units, signed_raw_string, ZERO_ADDRESS};

use super::{AddressPoolPnlSummary, AddressPoolPosition, PoolPnlTracker};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoolPnlConservationTotals {
    pub token_in_raw: U256,
    pub token_out_raw: U256,
    pub denom_in_raw: U256,
    pub denom_out_raw: U256,
    pub pool_token_in_raw: U256,
    pub pool_token_out_raw: U256,
    pub pool_denom_in_raw: U256,
    pub pool_denom_out_raw: U256,
    pub native_fee_raw: U256,
    #[serde(default, alias = "native_bribe_raw")]
    pub native_priority_fee_raw: U256,
    pub token_transfer_count: u64,
    pub denom_transfer_count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolPnlConservationSummary {
    pub token_delta_raw: String,
    pub denom_delta_raw: String,
    pub token_delta: Decimal,
    pub denom_delta: Decimal,
    pub token_is_conserved: bool,
    pub denom_is_conserved: bool,
    pub token_transfer_count: u64,
    pub denom_transfer_count: u64,
    pub pool_token_delta_raw: String,
    pub pool_denom_delta_raw: String,
    pub pool_token_delta: Decimal,
    pub pool_denom_delta: Decimal,
}

impl PoolPnlTracker {
    pub fn address_summaries(
        &self,
        mark_price_denom_per_token: Option<f64>,
    ) -> Vec<AddressPoolPnlSummary> {
        self.positions
            .values()
            .map(|position| self.summary_for_position(position, mark_price_denom_per_token))
            .collect()
    }

    pub fn top_positions_by_denom_volume(
        &self,
        limit: usize,
        include_pool_and_zero: bool,
        mark_price_denom_per_token: Option<f64>,
    ) -> Vec<AddressPoolPnlSummary> {
        let mut positions = self
            .positions
            .values()
            .filter(|position| {
                include_pool_and_zero
                    || (position.address != self.pool_address && position.address != ZERO_ADDRESS)
            })
            .collect::<Vec<_>>();
        positions.sort_by(|left, right| {
            let left_volume = left.denom_in_raw.saturating_add(left.denom_out_raw);
            let right_volume = right.denom_in_raw.saturating_add(right.denom_out_raw);
            right_volume.cmp(&left_volume)
        });
        positions
            .into_iter()
            .take(limit)
            .map(|position| self.summary_for_position(position, mark_price_denom_per_token))
            .collect()
    }

    pub fn conservation_summary(&self) -> PoolPnlConservationSummary {
        PoolPnlConservationSummary {
            token_delta_raw: signed_raw_string(
                self.conservation.token_in_raw,
                self.conservation.token_out_raw,
            ),
            denom_delta_raw: signed_raw_string(
                self.conservation.denom_in_raw,
                self.conservation.denom_out_raw,
            ),
            token_delta: scaled_signed_balance(
                self.conservation.token_in_raw,
                self.conservation.token_out_raw,
                self.token_decimals,
            ),
            denom_delta: scaled_signed_balance(
                self.conservation.denom_in_raw,
                self.conservation.denom_out_raw,
                self.denom_decimals,
            ),
            token_is_conserved: self.conservation.token_in_raw == self.conservation.token_out_raw,
            denom_is_conserved: self.conservation.denom_in_raw == self.conservation.denom_out_raw,
            token_transfer_count: self.conservation.token_transfer_count,
            denom_transfer_count: self.conservation.denom_transfer_count,
            pool_token_delta_raw: signed_raw_string(
                self.conservation.pool_token_in_raw,
                self.conservation.pool_token_out_raw,
            ),
            pool_denom_delta_raw: signed_raw_string(
                self.conservation.pool_denom_in_raw,
                self.conservation.pool_denom_out_raw,
            ),
            pool_token_delta: scaled_signed_balance(
                self.conservation.pool_token_in_raw,
                self.conservation.pool_token_out_raw,
                self.token_decimals,
            ),
            pool_denom_delta: scaled_signed_balance(
                self.conservation.pool_denom_in_raw,
                self.conservation.pool_denom_out_raw,
                self.denom_decimals,
            ),
        }
    }

    fn summary_for_position(
        &self,
        position: &AddressPoolPosition,
        mark_price_denom_per_token: Option<f64>,
    ) -> AddressPoolPnlSummary {
        let token_balance = scaled_signed_balance(
            position.token_in_raw,
            position.token_out_raw,
            self.token_decimals,
        );
        let denom_cashflow = scaled_signed_balance(
            position.denom_in_raw,
            position.denom_out_raw,
            self.denom_decimals,
        );
        let native_fee = scaled_units(position.native_fee_raw, 18);
        let native_priority_fee = scaled_units(position.native_priority_fee_raw, 18);
        let marked_token_value_denom = mark_price_denom_per_token
            .and_then(|price| Decimal::try_from(price).ok())
            .map(|price| token_balance * price);
        let native_costs = if self.denom_tracks_native_eth() {
            native_fee + native_priority_fee
        } else {
            Decimal::ZERO
        };
        let pnl_proxy_denom =
            marked_token_value_denom.map(|marked| denom_cashflow + marked - native_costs);

        AddressPoolPnlSummary {
            address: position.address.clone(),
            token_balance_raw: signed_raw_string(position.token_in_raw, position.token_out_raw),
            denom_cashflow_raw: signed_raw_string(position.denom_in_raw, position.denom_out_raw),
            native_fee_raw: position.native_fee_raw.to_string(),
            native_priority_fee_raw: position.native_priority_fee_raw.to_string(),
            token_balance,
            denom_cashflow,
            native_fee,
            native_priority_fee,
            marked_token_value_denom,
            pnl_proxy_denom,
            token_in_raw: position.token_in_raw.to_string(),
            token_out_raw: position.token_out_raw.to_string(),
            denom_in_raw: position.denom_in_raw.to_string(),
            denom_out_raw: position.denom_out_raw.to_string(),
            first_block: position.first_block,
            latest_block: position.latest_block,
            movement_count: position.movement_count,
        }
    }
}
