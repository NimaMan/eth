use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::pools::PoolLifecycle;
use crate::token_activity::TokenBlockActivity;

use super::{ObservationTransactionSummary, TokenPoolObservationContext, TokenPoolObservationKey};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenPoolCurrentObservation {
    pub key: TokenPoolObservationKey,
    pub context: TokenPoolObservationContext,
    pub activity: ObservationBlockActivity,
    pub trading: ObservationPoolTradingState,
    pub transactions: Vec<ObservationTransactionSummary>,
}

impl TokenPoolCurrentObservation {
    pub fn as_of_block(&self) -> u64 {
        self.context.block_number
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObservationBlockActivity {
    pub tx_count: u32,
    pub token_transfer_count: u32,
    pub denom_transfer_count: u32,
    pub buy_volume_by_denom: BTreeMap<String, f64>,
    pub sell_volume_by_denom: BTreeMap<String, f64>,
    pub total_bribe_eth: f64,
}

impl ObservationBlockActivity {
    pub fn buy_volume_for_denom(&self, denom_address: &str) -> f64 {
        volume_for_denom(&self.buy_volume_by_denom, denom_address)
    }

    pub fn sell_volume_for_denom(&self, denom_address: &str) -> f64 {
        volume_for_denom(&self.sell_volume_by_denom, denom_address)
    }

    pub fn has_signal(&self) -> bool {
        self.tx_count > 0
            || self.token_transfer_count > 0
            || self.denom_transfer_count > 0
            || self
                .buy_volume_by_denom
                .values()
                .any(|amount| *amount > 0.0)
            || self
                .sell_volume_by_denom
                .values()
                .any(|amount| *amount > 0.0)
            || self.total_bribe_eth > 0.0
    }
}

impl From<&TokenBlockActivity> for ObservationBlockActivity {
    fn from(activity: &TokenBlockActivity) -> Self {
        Self {
            tx_count: activity.num_tx,
            token_transfer_count: activity.token_transfer_count,
            denom_transfer_count: activity.denom_transfer_count,
            buy_volume_by_denom: activity.buy_volume_by_denom.clone(),
            sell_volume_by_denom: activity.sell_volume_by_denom.clone(),
            total_bribe_eth: activity.total_bribe_eth,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObservationPoolTradingState {
    pub lifecycle: Option<PoolLifecycle>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub effective_can_buy: bool,
    pub effective_can_sell: bool,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub tax_check_block: Option<u64>,
    pub tax_check_tx: Option<String>,
    pub last_trading_failure_class: Option<String>,
}

impl ObservationPoolTradingState {
    pub fn is_tradeable(&self) -> bool {
        self.effective_can_buy && self.effective_can_sell
    }
}

fn volume_for_denom(volume: &BTreeMap<String, f64>, denom_address: &str) -> f64 {
    let denom = denom_address.trim().to_ascii_lowercase();
    volume
        .iter()
        .find(|(key, _)| key.trim().eq_ignore_ascii_case(&denom))
        .map(|(_, amount)| *amount)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_activity_converts_from_current_token_activity() {
        let mut source = TokenBlockActivity::new(10, Some(100));
        source.num_tx = 2;
        source.token_transfer_count = 3;
        source.denom_transfer_count = 4;
        source.buy_volume_by_denom.insert("0xC02A".to_string(), 1.5);
        source
            .sell_volume_by_denom
            .insert("0xc02a".to_string(), 0.5);
        source.total_bribe_eth = 0.02;

        let activity = ObservationBlockActivity::from(&source);

        assert_eq!(activity.tx_count, 2);
        assert_eq!(activity.token_transfer_count, 3);
        assert_eq!(activity.denom_transfer_count, 4);
        assert_eq!(activity.buy_volume_for_denom("0xc02a"), 1.5);
        assert_eq!(activity.sell_volume_for_denom("0xC02A"), 0.5);
        assert!(activity.has_signal());
    }

    #[test]
    fn trading_state_uses_effective_buy_and_sell() {
        let state = ObservationPoolTradingState {
            can_buy: true,
            can_sell: true,
            effective_can_buy: true,
            effective_can_sell: false,
            ..Default::default()
        };

        assert!(!state.is_tradeable());
    }
}
