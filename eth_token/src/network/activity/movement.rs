//! Token and denomination movement records.

use serde::{Deserialize, Serialize};

use crate::network::model::NetworkObservation;

/// Direction of a movement from the perspective of the tracked address.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementDirection {
    In,
    Out,
}

impl MovementDirection {
    pub fn signed_amount(self, amount: f64) -> f64 {
        match self {
            Self::In => amount,
            Self::Out => -amount,
        }
    }
}

/// Asset family for an address movement.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MovementAssetKind {
    Token,
    Native,
    WrappedNative,
    Stable,
    KnownDenom,
    LpToken,
    Other,
}

impl MovementAssetKind {
    pub fn is_denom(&self) -> bool {
        matches!(self, Self::Native | Self::WrappedNative | Self::Stable)
    }
}

/// One token, denomination, or LP movement for an address.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddressMovement {
    pub direction: MovementDirection,
    pub asset_kind: MovementAssetKind,
    pub asset: Option<String>,
    pub symbol: Option<String>,
    pub raw_amount: Option<String>,
    pub amount: f64,
    pub observation: NetworkObservation,
}

impl AddressMovement {
    pub fn new(
        direction: MovementDirection,
        asset_kind: MovementAssetKind,
        amount: f64,
        observation: NetworkObservation,
    ) -> Self {
        Self {
            direction,
            asset_kind,
            asset: None,
            symbol: None,
            raw_amount: None,
            amount: normalized_amount(amount),
            observation,
        }
    }

    pub fn token_in(amount: f64, observation: NetworkObservation) -> Self {
        Self::new(
            MovementDirection::In,
            MovementAssetKind::Token,
            amount,
            observation,
        )
    }

    pub fn token_out(amount: f64, observation: NetworkObservation) -> Self {
        Self::new(
            MovementDirection::Out,
            MovementAssetKind::Token,
            amount,
            observation,
        )
    }

    pub fn denom_in(amount: f64, observation: NetworkObservation) -> Self {
        Self::new(
            MovementDirection::In,
            MovementAssetKind::Native,
            amount,
            observation,
        )
    }

    pub fn denom_out(amount: f64, observation: NetworkObservation) -> Self {
        Self::new(
            MovementDirection::Out,
            MovementAssetKind::Native,
            amount,
            observation,
        )
    }

    pub fn with_asset(mut self, asset: impl Into<String>) -> Self {
        self.asset = Some(asset.into());
        self
    }

    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn with_raw_amount(mut self, raw_amount: impl Into<String>) -> Self {
        self.raw_amount = Some(raw_amount.into());
        self
    }

    pub fn signed_amount(&self) -> f64 {
        self.direction.signed_amount(self.amount)
    }
}

/// Running movement totals for one address.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AddressMovementTotals {
    pub token_in: f64,
    pub token_out: f64,
    pub denom_in: f64,
    pub denom_out: f64,
    pub lp_in: f64,
    pub lp_out: f64,
    pub other_in: f64,
    pub other_out: f64,
    pub token_in_count: u64,
    pub token_out_count: u64,
    pub denom_in_count: u64,
    pub denom_out_count: u64,
    pub lp_in_count: u64,
    pub lp_out_count: u64,
    pub other_in_count: u64,
    pub other_out_count: u64,
}

impl AddressMovementTotals {
    pub fn record(&mut self, movement: &AddressMovement) {
        let amount = normalized_amount(movement.amount);
        match (&movement.asset_kind, movement.direction) {
            (MovementAssetKind::Token, MovementDirection::In) => {
                self.token_in += amount;
                self.token_in_count += 1;
            }
            (MovementAssetKind::Token, MovementDirection::Out) => {
                self.token_out += amount;
                self.token_out_count += 1;
            }
            (MovementAssetKind::LpToken, MovementDirection::In) => {
                self.lp_in += amount;
                self.lp_in_count += 1;
            }
            (MovementAssetKind::LpToken, MovementDirection::Out) => {
                self.lp_out += amount;
                self.lp_out_count += 1;
            }
            (kind, MovementDirection::In) if kind.is_denom() => {
                self.denom_in += amount;
                self.denom_in_count += 1;
            }
            (kind, MovementDirection::Out) if kind.is_denom() => {
                self.denom_out += amount;
                self.denom_out_count += 1;
            }
            (_, MovementDirection::In) => {
                self.other_in += amount;
                self.other_in_count += 1;
            }
            (_, MovementDirection::Out) => {
                self.other_out += amount;
                self.other_out_count += 1;
            }
        }
    }

    pub fn token_balance(&self) -> f64 {
        round_near_zero(self.token_in - self.token_out)
    }

    pub fn denom_balance(&self) -> f64 {
        self.denom_in - self.denom_out
    }

    pub fn lp_balance(&self) -> f64 {
        round_near_zero(self.lp_in - self.lp_out)
    }

    pub fn buy_count(&self) -> u64 {
        self.denom_out_count
    }

    pub fn sell_count(&self) -> u64 {
        self.denom_in_count
    }

    pub fn tx_count_proxy(&self) -> u64 {
        self.buy_count() + self.sell_count()
    }
}

pub fn normalized_amount(amount: f64) -> f64 {
    if amount.is_finite() && amount > 0.0 {
        amount
    } else {
        0.0
    }
}

pub fn round_near_zero(value: f64) -> f64 {
    if value.abs() < 0.000001 {
        0.0
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(block_number: u64) -> NetworkObservation {
        NetworkObservation::new(block_number)
    }

    #[test]
    fn totals_track_token_and_denom_sides() {
        let mut totals = AddressMovementTotals::default();
        totals.record(&AddressMovement::token_in(100.0, obs(1)));
        totals.record(&AddressMovement::token_out(25.0, obs(2)));
        totals.record(&AddressMovement::denom_out(1.5, obs(1)));
        totals.record(&AddressMovement::denom_in(0.25, obs(2)));

        assert_eq!(totals.token_balance(), 75.0);
        assert_eq!(totals.denom_balance(), -1.25);
        assert_eq!(totals.buy_count(), 1);
        assert_eq!(totals.sell_count(), 1);
    }

    #[test]
    fn known_denom_is_not_pool_denom_for_network_pnl() {
        let mut totals = AddressMovementTotals::default();
        totals.record(&AddressMovement::new(
            MovementDirection::In,
            MovementAssetKind::KnownDenom,
            42.0,
            obs(1),
        ));

        assert_eq!(totals.denom_in, 0.0);
        assert_eq!(totals.other_in, 42.0);
        assert_eq!(totals.buy_count(), 0);
        assert_eq!(totals.sell_count(), 0);
    }
}
