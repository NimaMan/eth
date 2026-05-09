use alloy_primitives::Address;
use eth_alpha_core::{
    market::MarketSnapshotRef, MarketEvent, PortfolioState, Result, Strategy, StrategyContext,
    StrategyDecision, StrategyName,
};

struct HoldStrategy;

impl Strategy for HoldStrategy {
    fn name(&self) -> StrategyName {
        StrategyName("hold".to_string())
    }

    fn on_market_event(
        &mut self,
        _ctx: &StrategyContext<'_>,
        _event: &MarketEvent,
    ) -> Result<StrategyDecision> {
        Ok(StrategyDecision::Hold)
    }
}

#[test]
fn strategy_returns_decision_without_mutating_position_state() {
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: Address::ZERO,
        pool_address: None,
        token: None,
        pool: None,
    };
    let portfolio = PortfolioState::default();
    let ctx = StrategyContext {
        market: &market,
        portfolio: &portfolio,
        active_risks: &[],
    };

    let mut strategy = HoldStrategy;
    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::BlockCompleted {
                block_number: 1,
                updated_tokens: 0,
                updated_pools: 0,
            },
        )
        .unwrap();

    assert_eq!(decision, StrategyDecision::Hold);
}
