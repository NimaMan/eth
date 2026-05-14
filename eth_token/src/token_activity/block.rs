use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenBlockActivity {
    pub block_number: u64,
    pub timestamp: Option<u64>,
    pub num_tx: u32,
    pub token_transfer_count: u32,
    pub denom_transfer_count: u32,
    pub buy_volume_by_denom: BTreeMap<String, f64>,
    pub sell_volume_by_denom: BTreeMap<String, f64>,
    pub total_bribe_eth: f64,
}

impl TokenBlockActivity {
    pub fn new(block_number: u64, timestamp: Option<u64>) -> Self {
        Self {
            block_number,
            timestamp,
            ..Default::default()
        }
    }

    pub fn refresh_timestamp(&mut self, timestamp: Option<u64>) {
        if timestamp.is_some() {
            self.timestamp = timestamp;
        }
    }
}
