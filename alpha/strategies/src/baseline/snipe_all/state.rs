use std::collections::{HashMap, HashSet};

use alloy_primitives::U256;
use eth_alpha_core::ids::{BlockNumber, PoolAddress, PositionId};

#[derive(Clone, Debug, Default)]
pub struct SnipeAllState {
    bought_pools: HashSet<PoolAddress>,
    active_hold_blocks: HashMap<PositionId, ActiveHoldCounter>,
    restored_entry_bankroll: RestoredEntryBankroll,
}

impl SnipeAllState {
    pub fn with_bought_pools(pools: impl IntoIterator<Item = PoolAddress>) -> Self {
        Self {
            bought_pools: pools.into_iter().collect(),
            active_hold_blocks: HashMap::new(),
            restored_entry_bankroll: RestoredEntryBankroll::default(),
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
            restored_entry_bankroll: RestoredEntryBankroll::default(),
        }
    }

    pub fn with_restored_entry_bankroll(mut self, bankroll: RestoredEntryBankroll) -> Self {
        self.restored_entry_bankroll = bankroll;
        self
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

    pub fn bought_pool_count(&self) -> usize {
        self.bought_pools.len()
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

    pub fn restored_entry_bankroll(&self) -> &RestoredEntryBankroll {
        &self.restored_entry_bankroll
    }
}

#[derive(Clone, Debug, Default)]
struct ActiveHoldCounter {
    count: u64,
    last_block: Option<BlockNumber>,
}

#[derive(Clone, Debug, Default)]
pub struct RestoredEntryBankroll {
    accounted_pools: HashSet<PoolAddress>,
    spent_wei: U256,
    recovered_wei: U256,
}

impl RestoredEntryBankroll {
    pub fn record_accounted_pool(&mut self, pool: PoolAddress) {
        self.accounted_pools.insert(pool);
    }

    pub fn record_position_result(
        &mut self,
        pool: PoolAddress,
        spent_wei: U256,
        recovered_wei: U256,
    ) {
        self.accounted_pools.insert(pool);
        self.spent_wei = self.spent_wei.saturating_add(spent_wei);
        self.recovered_wei = self.recovered_wei.saturating_add(recovered_wei);
    }

    pub fn apply_to(&self, available: U256) -> U256 {
        available
            .saturating_sub(self.spent_wei)
            .saturating_add(self.recovered_wei)
    }

    pub fn accounts_for(&self, pool: &PoolAddress) -> bool {
        self.accounted_pools.contains(pool)
    }

    pub fn accounted_pool_count(&self) -> usize {
        self.accounted_pools.len()
    }

    pub fn spent_wei(&self) -> U256 {
        self.spent_wei
    }

    pub fn recovered_wei(&self) -> U256 {
        self.recovered_wei
    }
}
