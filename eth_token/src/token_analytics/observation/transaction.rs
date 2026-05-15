use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{ObservationTransactionClassification, ObservationTransactionType};
use crate::token_activity::TokenTransactionActivity;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObservationTransactionSummary {
    pub tx_hash: String,
    pub block_number: u64,
    pub timestamp: Option<u64>,
    pub maker: Option<String>,
    pub tx_types: Vec<ObservationTransactionType>,
    pub classification: ObservationTransactionClassification,
    pub affects_token: bool,
    pub affects_pool: bool,
    pub token_transfer_count: u32,
    pub denom_transfer_count: u32,
    pub token_approval_count: u32,
    pub lp_approval_count: u32,
    pub buy_volume_by_denom: BTreeMap<String, f64>,
    pub sell_volume_by_denom: BTreeMap<String, f64>,
    pub total_bribe_eth: f64,
}

impl ObservationTransactionSummary {
    pub fn add_type(&mut self, tx_type: ObservationTransactionType) {
        if !self.tx_types.contains(&tx_type) {
            self.affects_token |= tx_type.affects_token();
            self.affects_pool |= tx_type.affects_pool();
            self.tx_types.push(tx_type);
        }
    }

    pub fn refresh_classification(&mut self) {
        self.classification =
            ObservationTransactionClassification::from_types(self.tx_types.clone());
        self.affects_token = self.classification.affects_token;
        self.affects_pool = self.classification.affects_pool;
    }

    pub fn infer_types(&mut self) {
        if self.token_transfer_count > 0 {
            self.add_type(ObservationTransactionType::TokenTransfer);
        }
        if self.denom_transfer_count > 0 {
            self.add_type(ObservationTransactionType::DenomTransfer);
        }
        if self.token_approval_count > 0 {
            self.add_type(ObservationTransactionType::TokenApproval);
        }
        if self.lp_approval_count > 0 {
            self.add_type(ObservationTransactionType::LpApproval);
        }
        if self
            .buy_volume_by_denom
            .values()
            .any(|amount| *amount > 0.0)
            || self
                .sell_volume_by_denom
                .values()
                .any(|amount| *amount > 0.0)
        {
            self.add_type(ObservationTransactionType::Swap);
        }
        if self.total_bribe_eth > 0.0 {
            self.add_type(ObservationTransactionType::Bribe);
        }
        if self.tx_types.is_empty() {
            self.add_type(ObservationTransactionType::Other);
        }
        self.refresh_classification();
    }
}

impl From<&TokenTransactionActivity> for ObservationTransactionSummary {
    fn from(activity: &TokenTransactionActivity) -> Self {
        let mut summary = Self {
            tx_hash: activity.tx_hash.clone(),
            block_number: activity.block_number,
            timestamp: activity.timestamp,
            maker: activity.maker.clone(),
            token_transfer_count: activity.token_transfer_count,
            denom_transfer_count: activity.denom_transfer_count,
            buy_volume_by_denom: activity.buy_volume_by_denom.clone(),
            sell_volume_by_denom: activity.sell_volume_by_denom.clone(),
            total_bribe_eth: activity.total_bribe_eth,
            ..Default::default()
        };
        summary.infer_types();
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_summary_infers_swap_transfer_and_bribe_types() {
        let mut tx = TokenTransactionActivity::new("0xA", 10, Some(100), Some("0xMaker".into()));
        tx.token_transfer_count = 1;
        tx.denom_transfer_count = 2;
        tx.buy_volume_by_denom.insert("0xC02A".to_string(), 1.0);
        tx.total_bribe_eth = 0.01;

        let summary = ObservationTransactionSummary::from(&tx);

        assert!(summary.tx_types.contains(&ObservationTransactionType::Swap));
        assert!(summary
            .tx_types
            .contains(&ObservationTransactionType::TokenTransfer));
        assert!(summary
            .tx_types
            .contains(&ObservationTransactionType::DenomTransfer));
        assert!(summary
            .tx_types
            .contains(&ObservationTransactionType::Bribe));
        assert!(summary.affects_token);
        assert!(summary.affects_pool);
        assert_eq!(
            summary.classification.primary_type,
            ObservationTransactionType::Swap
        );
        assert_eq!(summary.classification.label, "Pool swap");
    }

    #[test]
    fn transaction_summary_infers_token_approval_type() {
        let mut summary = ObservationTransactionSummary {
            tx_hash: "0xA".to_string(),
            block_number: 10,
            token_approval_count: 3,
            ..Default::default()
        };

        summary.infer_types();

        assert!(summary
            .tx_types
            .contains(&ObservationTransactionType::TokenApproval));
        assert!(summary.affects_token);
        assert!(!summary.affects_pool);
        assert_eq!(
            summary.classification.primary_type,
            ObservationTransactionType::TokenApproval
        );
        assert_eq!(summary.classification.label, "Token approval");
    }
}
