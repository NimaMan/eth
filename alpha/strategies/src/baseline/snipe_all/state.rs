use std::collections::HashSet;

use eth_alpha_core::ids::PoolAddress;

#[derive(Clone, Debug, Default)]
pub struct SnipeAllState {
    bought_pools: HashSet<PoolAddress>,
}

impl SnipeAllState {
    pub fn with_bought_pools(pools: impl IntoIterator<Item = PoolAddress>) -> Self {
        Self {
            bought_pools: pools.into_iter().collect(),
        }
    }

    pub fn has_bought(&self, pool: &PoolAddress) -> bool {
        self.bought_pools.contains(pool)
    }

    pub fn mark_bought(&mut self, pool: PoolAddress) {
        self.bought_pools.insert(pool);
    }

    pub fn bought_pools(&self) -> &HashSet<PoolAddress> {
        &self.bought_pools
    }
}
