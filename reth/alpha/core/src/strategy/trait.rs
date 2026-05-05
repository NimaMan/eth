use crate::{
    error::Result,
    ids::StrategyName,
    market::MarketEvent,
    strategy::{StrategyContext, StrategyDecision},
};

pub trait Strategy: Send {
    fn name(&self) -> StrategyName;

    fn on_market_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        event: &MarketEvent,
    ) -> Result<StrategyDecision>;
}
