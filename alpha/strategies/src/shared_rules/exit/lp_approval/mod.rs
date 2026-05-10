use eth_alpha_core::{
    ids::{PoolAddress, StrategyName},
    risk::{RiskEvent, RiskKind},
    StrategyContext,
};

use crate::baseline::snipe_all::rule::RuleDecision;

pub const RULE_NAME: &str = "exit.lp_approval";

/// Exit when an `LpApproval` risk event targets a pool with an open position.
pub fn evaluate(
    ctx: &StrategyContext<'_>,
    strategy_name: &StrategyName,
    event: &RiskEvent,
) -> RuleDecision {
    if event.kind != RiskKind::LpApproval {
        return RuleDecision::hold(RULE_NAME, "risk event is not lp approval");
    }

    let Some(pool_address) = event
        .pool_address
        .clone()
        .or_else(|| ctx.market.pool_address.clone())
    else {
        return RuleDecision::hold(RULE_NAME, "lp-approval event has no pool");
    };

    if !has_open_matching_position(ctx, strategy_name, &pool_address) {
        return RuleDecision::hold(RULE_NAME, "no open matching position");
    }

    RuleDecision::Exit { rule: RULE_NAME }
}

fn has_open_matching_position(
    ctx: &StrategyContext<'_>,
    strategy_name: &StrategyName,
    pool_address: &PoolAddress,
) -> bool {
    ctx.portfolio.positions.values().any(|position| {
        &position.key.pool_address == pool_address
            && position.key.strategy_name == *strategy_name
            && !position.state.is_terminal()
    })
}
