use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::pnl::common::{normalize_address, normalize_address_string, WETH_ADDRESS};

use super::{AddressPoolPosition, PoolPnlConservationTotals, PoolPnlEntry};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolPnlTracker {
    pub pool_address: String,
    pub token_address: String,
    pub denom_address: String,
    pub token_decimals: u8,
    pub denom_decimals: u8,
    pub history_limit: usize,
    #[serde(default)]
    pub positions: BTreeMap<String, AddressPoolPosition>,
    #[serde(default)]
    pub conservation: PoolPnlConservationTotals,
    #[serde(default)]
    pub recent_entries: Vec<PoolPnlEntry>,
    #[serde(default)]
    pub tx_count: u64,
    pub latest_block_number: Option<u64>,
    pub latest_block_timestamp: Option<u64>,
}

impl PoolPnlTracker {
    pub fn new(
        pool_address: impl Into<String>,
        token_address: impl Into<String>,
        denom_address: impl Into<String>,
        token_decimals: u8,
        denom_decimals: u8,
        history_limit: usize,
    ) -> Self {
        Self {
            pool_address: normalize_address_string(pool_address.into()),
            token_address: normalize_address_string(token_address.into()),
            denom_address: normalize_address_string(denom_address.into()),
            token_decimals,
            denom_decimals,
            history_limit,
            positions: BTreeMap::new(),
            conservation: PoolPnlConservationTotals::default(),
            recent_entries: Vec::new(),
            tx_count: 0,
            latest_block_number: None,
            latest_block_timestamp: None,
        }
    }

    pub fn position(&self, address: impl AsRef<str>) -> Option<&AddressPoolPosition> {
        self.positions.get(&normalize_address(address))
    }

    pub(crate) fn push_entry(&mut self, entry: PoolPnlEntry) {
        self.recent_entries.push(entry);
        self.prune_history();
    }

    pub(crate) fn prune_history(&mut self) {
        if self.history_limit == 0 || self.recent_entries.len() <= self.history_limit {
            return;
        }
        let excess = self.recent_entries.len() - self.history_limit;
        self.recent_entries.drain(0..excess);
    }

    pub(crate) fn denom_tracks_native_eth(&self) -> bool {
        self.denom_address == WETH_ADDRESS
    }
}
