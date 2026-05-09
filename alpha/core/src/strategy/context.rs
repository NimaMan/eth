use crate::{market::MarketSnapshotRef, portfolio::PortfolioState, risk::RiskEvent};

#[derive(Clone, Debug)]
pub struct StrategyContext<'a> {
    pub market: &'a MarketSnapshotRef,
    pub portfolio: &'a PortfolioState,
    pub active_risks: &'a [RiskEvent],
}
