use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenTransactionActivity {
    pub tx_hash: String,
    pub block_number: u64,
    pub timestamp: Option<u64>,
    pub maker: Option<String>,
    pub token_transfer_count: u32,
    pub token_transfer_volume: f64,
    pub denom_transfer_count: u32,
    pub buy_volume_by_denom: BTreeMap<String, f64>,
    pub sell_volume_by_denom: BTreeMap<String, f64>,
    pub total_bribe_eth: f64,
}

impl TokenTransactionActivity {
    pub fn new(
        tx_hash: impl Into<String>,
        block_number: u64,
        timestamp: Option<u64>,
        maker: Option<String>,
    ) -> Self {
        Self {
            tx_hash: tx_hash.into(),
            block_number,
            timestamp,
            maker,
            ..Default::default()
        }
    }

    pub fn refresh_context(&mut self, timestamp: Option<u64>, maker: Option<String>) {
        if timestamp.is_some() {
            self.timestamp = timestamp;
        }
        if maker.is_some() {
            self.maker = maker;
        }
    }
}
