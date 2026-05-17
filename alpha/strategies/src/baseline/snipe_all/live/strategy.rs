use eth_alpha_core::{
    ids::{PoolAddress, StrategyName},
    market::MarketEvent,
    risk::RiskEvent,
    Result, Strategy, StrategyContext, StrategyDecision,
};

use crate::baseline::snipe_all::{LiveSnipeAllConfig, SnipeAllStrategy};

#[derive(Clone, Debug)]
pub struct LiveSnipeAllStrategy {
    inner: SnipeAllStrategy,
}

impl LiveSnipeAllStrategy {
    pub fn new(config: LiveSnipeAllConfig) -> Self {
        Self {
            inner: SnipeAllStrategy::new(config.strategy),
        }
    }

    pub fn with_bought_pools(
        config: LiveSnipeAllConfig,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
    ) -> Self {
        Self {
            inner: SnipeAllStrategy::with_bought_pools(config.strategy, bought_pools),
        }
    }

    pub fn inner(&self) -> &SnipeAllStrategy {
        &self.inner
    }
}

impl Strategy for LiveSnipeAllStrategy {
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

#[cfg(test)]
mod tests {
    use eth_alpha_core::ids::StrategyName;

    use super::*;
    use crate::baseline::snipe_all::SnipeAllConfig;

    #[test]
    fn live_wrapper_uses_regular_strategy_identity() {
        let strategy = LiveSnipeAllStrategy::new(LiveSnipeAllConfig::new(SnipeAllConfig {
            strategy_name: StrategyName("snipe-all-live".to_string()),
            ..SnipeAllConfig::default()
        }));

        assert_eq!(strategy.name(), StrategyName("snipe-all-live".to_string()));
    }
}
