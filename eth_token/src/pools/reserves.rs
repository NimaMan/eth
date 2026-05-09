use serde::{Deserialize, Serialize};

use crate::utils::append_with_history_limit;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReserveSnapshot {
    pub block_number: u64,
    pub tx_hash: String,
    pub denom_reserve: f64,
    pub token_reserve: f64,
    pub price: f64,
    pub timestamp: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolReserveTracker {
    pub pool_address: String,
    pub denom_address: String,
    pub token_address: Option<String>,
    pub pool_type: Option<String>,
    pub history_limit: usize,
    pub reserve_history: Vec<ReserveSnapshot>,
    pub latest_snapshot: Option<ReserveSnapshot>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub scam_block: Option<u64>,
    pub scam_tx_hash: Option<String>,
    pub denom_threshold: Option<f64>,
    pub threshold_unit: Option<String>,
}

impl PoolReserveTracker {
    pub fn new(
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        token_address: Option<impl Into<String>>,
        pool_type: Option<impl Into<String>>,
        history_limit: usize,
    ) -> Self {
        Self {
            pool_address: pool_address.into(),
            denom_address: denom_address.into(),
            token_address: token_address.map(Into::into),
            pool_type: pool_type.map(Into::into),
            history_limit,
            reserve_history: Vec::new(),
            latest_snapshot: None,
            is_scam: false,
            scam_label: None,
            scam_block: None,
            scam_tx_hash: None,
            denom_threshold: None,
            threshold_unit: None,
        }
    }

    pub fn with_threshold(mut self, threshold: f64, unit: impl Into<String>) -> Self {
        self.denom_threshold = Some(threshold);
        self.threshold_unit = Some(unit.into());
        self
    }

    pub fn update_reserves(
        &mut self,
        denom_reserve: f64,
        token_reserve: f64,
        price: f64,
        block_number: u64,
        timestamp: u64,
        tx_hash: impl Into<String>,
    ) {
        let snapshot = ReserveSnapshot {
            block_number,
            tx_hash: tx_hash.into(),
            denom_reserve,
            token_reserve,
            price,
            timestamp,
        };

        append_with_history_limit(
            &mut self.reserve_history,
            snapshot.clone(),
            self.history_limit,
        );
        self.latest_snapshot = Some(snapshot.clone());
        self.check_for_liquidity_removal(&snapshot);
    }

    pub fn latest_price(&self) -> Option<f64> {
        self.latest_snapshot.as_ref().map(|snapshot| snapshot.price)
    }

    pub fn latest_reserves(&self) -> (f64, f64) {
        self.latest_snapshot
            .as_ref()
            .map(|snapshot| (snapshot.denom_reserve, snapshot.token_reserve))
            .unwrap_or((0.0, 0.0))
    }

    pub fn price_history(&self) -> Vec<(String, u64, f64)> {
        self.reserve_history
            .iter()
            .map(|snapshot| {
                (
                    snapshot.tx_hash.clone(),
                    snapshot.block_number,
                    snapshot.price,
                )
            })
            .collect()
    }

    pub fn initial_price(&self) -> Option<f64> {
        self.reserve_history.first().map(|snapshot| snapshot.price)
    }

    pub fn price_ratio_to_initial(&self) -> Option<f64> {
        let initial = self.initial_price()?;
        let latest = self.latest_price()?;
        if initial > 0.0 {
            Some(latest / initial)
        } else {
            None
        }
    }

    fn check_for_liquidity_removal(&mut self, snapshot: &ReserveSnapshot) {
        let Some(threshold) = self.denom_threshold else {
            return;
        };

        if snapshot.denom_reserve < threshold {
            let unit = self.threshold_unit.as_deref().unwrap_or("units");
            self.is_scam = true;
            self.scam_label = Some(format!("liquidity_removal ({unit}<{threshold})"));
            self.scam_block = Some(snapshot.block_number);
            self.scam_tx_hash = Some(snapshot.tx_hash.clone());
        } else if self.is_scam {
            self.is_scam = false;
            self.scam_label = None;
            self.scam_block = None;
            self.scam_tx_hash = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_reserves_records_bounded_history() {
        let mut tracker = PoolReserveTracker::new("pool", "denom", Some("token"), Some("V2"), 2);

        tracker.update_reserves(10.0, 100.0, 0.1, 1, 11, "tx1");
        tracker.update_reserves(11.0, 100.0, 0.11, 2, 12, "tx2");
        tracker.update_reserves(12.0, 100.0, 0.12, 3, 13, "tx3");

        assert_eq!(tracker.reserve_history.len(), 2);
        assert_eq!(tracker.reserve_history[0].tx_hash, "tx2");
        assert_eq!(tracker.latest_price(), Some(0.12));
        assert_eq!(tracker.latest_reserves(), (12.0, 100.0));
    }

    #[test]
    fn threshold_marks_and_clears_liquidity_removal_state() {
        let mut tracker = PoolReserveTracker::new("pool", "denom", Some("token"), Some("V2"), 10)
            .with_threshold(0.05, "ETH");

        tracker.update_reserves(0.01, 100.0, 0.0001, 1, 11, "tx1");
        assert!(tracker.is_scam);
        assert_eq!(tracker.scam_block, Some(1));
        assert_eq!(tracker.scam_tx_hash.as_deref(), Some("tx1"));

        tracker.update_reserves(0.10, 100.0, 0.001, 2, 12, "tx2");
        assert!(!tracker.is_scam);
        assert!(tracker.scam_label.is_none());
    }
}
