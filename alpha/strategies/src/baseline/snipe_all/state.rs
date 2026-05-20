use std::collections::{HashMap, HashSet};

use eth_alpha_core::ids::{BlockNumber, PoolAddress, PositionId};

#[derive(Clone, Debug, Default)]
pub struct SnipeAllState {
    bought_pools: HashSet<PoolAddress>,
    active_hold_blocks: HashMap<PositionId, ActiveHoldCounter>,
}

impl SnipeAllState {
    pub fn with_bought_pools(pools: impl IntoIterator<Item = PoolAddress>) -> Self {
        Self {
            bought_pools: pools.into_iter().collect(),
            active_hold_blocks: HashMap::new(),
        }
    }

    pub fn with_bought_pools_and_active_hold_blocks(
        pools: impl IntoIterator<Item = PoolAddress>,
        active_hold_blocks: impl IntoIterator<Item = (PositionId, u64, Option<BlockNumber>)>,
    ) -> Self {
        Self {
            bought_pools: pools.into_iter().collect(),
            active_hold_blocks: active_hold_blocks
                .into_iter()
                .map(|(position_id, count, last_block)| {
                    (position_id, ActiveHoldCounter { count, last_block })
                })
                .collect(),
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

    pub fn observe_active_hold_block(
        &mut self,
        position_id: &PositionId,
        block_number: BlockNumber,
    ) -> u64 {
        let counter = self
            .active_hold_blocks
            .entry(position_id.clone())
            .or_default();
        if counter.last_block != Some(block_number) {
            counter.count = counter.count.saturating_add(1);
            counter.last_block = Some(block_number);
        }
        counter.count
    }

    pub fn active_hold_block_count(&self, position_id: &PositionId) -> u64 {
        self.active_hold_blocks
            .get(position_id)
            .map(|counter| counter.count)
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default)]
struct ActiveHoldCounter {
    count: u64,
    last_block: Option<BlockNumber>,
}
