use eth_alpha_core::{
    ids::{BlockNumber, PoolAddress, PositionId, StrategyName},
    market::MarketEvent,
    risk::RiskEvent,
    Result, Strategy, StrategyContext, StrategyDecision,
};

use crate::baseline::snipe_all::{RestoredEntryBankroll, SnipeAllStrategy};

use super::Alpha11Config;

#[derive(Clone, Debug)]
pub struct Alpha11Strategy {
    inner: SnipeAllStrategy,
}

impl Alpha11Strategy {
    pub fn new(config: Alpha11Config) -> Self {
        Self {
            inner: SnipeAllStrategy::new(config.snipe_all),
        }
    }

    pub fn with_bought_pools(
        config: Alpha11Config,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
    ) -> Self {
        Self {
            inner: SnipeAllStrategy::with_bought_pools(config.snipe_all, bought_pools),
        }
    }

    pub fn with_restored_state(
        config: Alpha11Config,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
        active_hold_blocks: impl IntoIterator<Item = (PositionId, u64, Option<BlockNumber>)>,
    ) -> Self {
        Self {
            inner: SnipeAllStrategy::with_restored_state(
                config.snipe_all,
                bought_pools,
                active_hold_blocks,
            ),
        }
    }

    pub fn with_restored_runtime_state(
        config: Alpha11Config,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
        active_hold_blocks: impl IntoIterator<Item = (PositionId, u64, Option<BlockNumber>)>,
        restored_entry_bankroll: RestoredEntryBankroll,
    ) -> Self {
        Self {
            inner: SnipeAllStrategy::with_restored_runtime_state(
                config.snipe_all,
                bought_pools,
                active_hold_blocks,
                restored_entry_bankroll,
            ),
        }
    }

    pub fn inner(&self) -> &SnipeAllStrategy {
        &self.inner
    }
}

impl Strategy for Alpha11Strategy {
    fn name(&self) -> StrategyName {
        self.inner.name()
    }

    fn on_market_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        event: &MarketEvent,
    ) -> Result<StrategyDecision> {
        self.inner.on_market_event(ctx, event)
    }

    fn on_risk_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        event: &RiskEvent,
    ) -> Result<StrategyDecision> {
        self.inner.on_risk_event(ctx, event)
    }

    fn on_position_monitor(
        &mut self,
        ctx: &StrategyContext<'_>,
        block_number: u64,
    ) -> Result<Vec<StrategyDecision>> {
        self.inner.on_position_monitor(ctx, block_number)
    }
}
