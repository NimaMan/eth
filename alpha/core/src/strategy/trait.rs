use crate::{
    error::Result,
    ids::StrategyName,
    market::MarketEvent,
    risk::RiskEvent,
    strategy::{StrategyContext, StrategyDecision},
};

pub trait Strategy: Send {
    fn name(&self) -> StrategyName;

    fn on_market_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        event: &MarketEvent,
    ) -> Result<StrategyDecision>;

    fn on_risk_event(
        &mut self,
        _ctx: &StrategyContext<'_>,
        _event: &RiskEvent,
    ) -> Result<StrategyDecision> {
        Ok(StrategyDecision::Hold)
    }

    fn on_position_monitor(
        &mut self,
        _ctx: &StrategyContext<'_>,
        _block_number: u64,
    ) -> Result<Vec<StrategyDecision>> {
        Ok(Vec::new())
    }
}
