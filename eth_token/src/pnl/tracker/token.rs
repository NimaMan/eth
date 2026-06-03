use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

use crate::pnl::common::normalize_address;

use super::PoolPnlTracker;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenPnlTracker {
    #[serde(default)]
    pools: BTreeMap<String, PoolPnlTracker>,
}

impl TokenPnlTracker {
    pub fn register_pool(
        &mut self,
        pool_address: impl AsRef<str>,
        token_address: impl AsRef<str>,
        denom_address: impl AsRef<str>,
        token_decimals: u8,
        denom_decimals: u8,
        history_limit: usize,
    ) {
        let pool_address = normalize_address(pool_address);
        let token_address = normalize_address(token_address);
        let denom_address = normalize_address(denom_address);
        self.pools
            .entry(pool_address.clone())
            .and_modify(|pool| {
                pool.token_address = token_address.clone();
                pool.denom_address = denom_address.clone();
                pool.token_decimals = token_decimals;
                pool.denom_decimals = denom_decimals;
                pool.history_limit = history_limit;
                pool.prune_history();
            })
            .or_insert_with(|| {
                PoolPnlTracker::new(
                    pool_address,
                    token_address,
                    denom_address,
                    token_decimals,
                    denom_decimals,
                    history_limit,
                )
            });
    }

    pub fn record_v2_pool_transaction(
        &mut self,
        pool_address: impl AsRef<str>,
        token_address: impl AsRef<str>,
        denom_address: impl AsRef<str>,
        token_decimals: u8,
        denom_decimals: u8,
        history_limit: usize,
        transaction: &ProcessedTransaction,
    ) {
        self.register_pool(
            pool_address.as_ref(),
            token_address.as_ref(),
            denom_address.as_ref(),
            token_decimals,
            denom_decimals,
            history_limit,
        );
        if let Some(pool) = self.pool_mut(pool_address.as_ref()) {
            pool.record_transaction(transaction);
        }
    }

    pub fn pool(&self, pool_address: impl AsRef<str>) -> Option<&PoolPnlTracker> {
        self.pools.get(&normalize_address(pool_address))
    }

    pub fn pool_mut(&mut self, pool_address: impl AsRef<str>) -> Option<&mut PoolPnlTracker> {
        self.pools.get_mut(&normalize_address(pool_address))
    }

    pub fn pools(&self) -> impl Iterator<Item = (&String, &PoolPnlTracker)> {
        self.pools.iter()
    }

    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }
}
