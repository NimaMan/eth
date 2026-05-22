use eth_alpha_core::{
    ids::{BlockNumber, PoolAddress, PositionId, StrategyName},
    market::MarketEvent,
    risk::RiskEvent,
    Result, Strategy, StrategyContext, StrategyDecision,
};

use crate::{
    alpha11::{Alpha11Strategy, LiveAlpha11Config},
    baseline::snipe_all::RestoredEntryBankroll,
};

#[derive(Clone, Debug)]
pub struct LiveAlpha11Strategy {
    inner: Alpha11Strategy,
}

impl LiveAlpha11Strategy {
    pub fn new(config: LiveAlpha11Config) -> Self {
        Self {
            inner: Alpha11Strategy::new(config.alpha11),
        }
    }

    pub fn with_bought_pools(
        config: LiveAlpha11Config,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
    ) -> Self {
        Self {
            inner: Alpha11Strategy::with_bought_pools(config.alpha11, bought_pools),
        }
    }

    pub fn with_restored_state(
        config: LiveAlpha11Config,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
        active_hold_blocks: impl IntoIterator<Item = (PositionId, u64, Option<BlockNumber>)>,
    ) -> Self {
        Self {
            inner: Alpha11Strategy::with_restored_state(
                config.alpha11,
                bought_pools,
                active_hold_blocks,
            ),
        }
    }

    pub fn with_restored_runtime_state(
        config: LiveAlpha11Config,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
        active_hold_blocks: impl IntoIterator<Item = (PositionId, u64, Option<BlockNumber>)>,
        restored_entry_bankroll: RestoredEntryBankroll,
    ) -> Self {
        Self {
            inner: Alpha11Strategy::with_restored_runtime_state(
                config.alpha11,
                bought_pools,
                active_hold_blocks,
                restored_entry_bankroll,
            ),
        }
    }

    pub fn inner(&self) -> &Alpha11Strategy {
        &self.inner
    }
}

impl Strategy for LiveAlpha11Strategy {
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
