use std::collections::HashSet;

use eth_alpha_core::ids::PoolAddress;

#[derive(Clone, Debug, Default)]
pub struct SnipeAllState {
    bought_pools: HashSet<PoolAddress>,
    exiting_pools: HashSet<PoolAddress>,
}

impl SnipeAllState {
    pub fn has_bought(&self, pool: PoolAddress) -> bool {
        self.bought_pools.contains(&pool)
    }

    pub fn mark_bought(&mut self, pool: PoolAddress) {
        self.bought_pools.insert(pool);
    }

    pub fn is_exiting(&self, pool: PoolAddress) -> bool {
        self.exiting_pools.contains(&pool)
    }

    pub fn mark_exiting(&mut self, pool: PoolAddress) {
        self.exiting_pools.insert(pool);
    }

    pub fn bought_pools(&self) -> &HashSet<PoolAddress> {
        &self.bought_pools
    }

    pub fn exiting_pools(&self) -> &HashSet<PoolAddress> {
        &self.exiting_pools
    }
}
