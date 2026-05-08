use eth_alpha_core::{
    ids::{PoolAddress, StrategyName},
    risk::{RiskEvent, RiskKind},
    StrategyContext,
};

use crate::snipe_all::{config::SnipeAllConfig, rule::RuleDecision, state::SnipeAllState};

pub const RULE_NAME: &str = "exit.liquidity_removal";

pub fn evaluate(
    config: &SnipeAllConfig,
    state: &SnipeAllState,
    ctx: &StrategyContext<'_>,
    strategy_name: &StrategyName,
    event: &RiskEvent,
) -> RuleDecision {
    if !config.sell_on_liquidity_removal {
        return RuleDecision::hold(RULE_NAME, "liquidity-removal exits disabled");
    }
    if event.kind != RiskKind::LiquidityRemoval {
        return RuleDecision::hold(RULE_NAME, "risk event is not liquidity removal");
    }

    let Some(pool_address) = event.pool_address.or(ctx.market.pool_address) else {
        return RuleDecision::hold(RULE_NAME, "liquidity-removal event has no pool");
    };
    if !state.has_bought(pool_address)
        && !has_open_matching_position(ctx, strategy_name, pool_address)
    {
        return RuleDecision::hold(RULE_NAME, "pool was not bought by strategy");
    }
    if state.is_exiting(pool_address) {
        return RuleDecision::hold(RULE_NAME, "pool exit already submitted");
    }
    if !has_open_matching_position(ctx, strategy_name, pool_address) {
        return RuleDecision::hold(RULE_NAME, "no open matching position");
    }

    RuleDecision::Exit { rule: RULE_NAME }
}

fn has_open_matching_position(
    ctx: &StrategyContext<'_>,
    strategy_name: &StrategyName,
    pool_address: PoolAddress,
) -> bool {
    ctx.portfolio.positions.values().any(|position| {
        position.key.pool_address == pool_address
            && position.key.strategy_name == *strategy_name
            && !position.state.is_terminal()
    })
}
