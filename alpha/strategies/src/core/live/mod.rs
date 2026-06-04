//! Generic live wrapper for the core engine.
//!
//! Live runtimes restore per-strategy state (seen pools, active-hold counters,
//! and entry bankroll) at startup. This wrapper is pure delegation to
//! [`StrategyEngine`]; it exists only to give live restore-construction a single
//! named type and to keep the live/backtest construction paths symmetric.

use eth_alpha_core::{
    ids::{BlockNumber, PoolAddress, PositionId, StrategyName},
    market::MarketEvent,
    risk::RiskEvent,
    Result, Strategy, StrategyContext, StrategyDecision,
};

use crate::core::{state::RestoredEntryBankroll, StrategyConfig, StrategyEngine};

#[derive(Clone, Debug)]
pub struct LiveStrategyConfig {
    pub strategy: StrategyConfig,
}

impl LiveStrategyConfig {
    pub fn new(strategy: StrategyConfig) -> Self {
        Self { strategy }
    }
}

impl Default for LiveStrategyConfig {
    fn default() -> Self {
        Self::new(StrategyConfig::default())
    }
}

#[derive(Clone, Debug)]
pub struct LiveStrategyEngine {
    inner: StrategyEngine,
}

impl LiveStrategyEngine {
    pub fn new(config: LiveStrategyConfig) -> Self {
        Self {
            inner: StrategyEngine::new(config.strategy),
        }
    }

    pub fn with_bought_pools(
        config: LiveStrategyConfig,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
    ) -> Self {
        Self {
            inner: StrategyEngine::with_bought_pools(config.strategy, bought_pools),
        }
    }

    pub fn with_restored_state(
        config: LiveStrategyConfig,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
        active_hold_blocks: impl IntoIterator<Item = (PositionId, u64, Option<BlockNumber>)>,
    ) -> Self {
        Self {
            inner: StrategyEngine::with_restored_state(
                config.strategy,
                bought_pools,
                active_hold_blocks,
            ),
        }
    }

    pub fn with_restored_runtime_state(
        config: LiveStrategyConfig,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
        active_hold_blocks: impl IntoIterator<Item = (PositionId, u64, Option<BlockNumber>)>,
        restored_entry_bankroll: RestoredEntryBankroll,
    ) -> Self {
        Self {
            inner: StrategyEngine::with_restored_runtime_state(
                config.strategy,
                bought_pools,
                active_hold_blocks,
                restored_entry_bankroll,
            ),
        }
    }

    pub fn inner(&self) -> &StrategyEngine {
        &self.inner
    }
}

impl Strategy for LiveStrategyEngine {
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

    #[test]
    fn live_wrapper_uses_regular_strategy_identity() {
        let strategy = LiveStrategyEngine::new(LiveStrategyConfig::new(StrategyConfig {
            strategy_name: StrategyName("core-live".to_string()),
            ..StrategyConfig::default()
        }));

        assert_eq!(strategy.name(), StrategyName("core-live".to_string()));
    }
}
