use eth_alpha_core::{
    ids::{PoolAddress, StrategyName},
    risk::{RiskEvent, RiskKind, RISK_SOURCE_MEMPOOL_SIGNAL},
    StrategyContext,
};

use crate::baseline::snipe_all::rule::RuleDecision;

pub const RULE_NAME: &str = "exit.liquidity_removal";
pub const MEMPOOL_SIGNAL_RULE_NAME: &str = "exit.mempool_liquidity_removal_signal";

/// Exit when a mined or mempool liquidity-removal risk event targets a pool
/// with an open position.
pub fn evaluate(
    ctx: &StrategyContext<'_>,
    strategy_name: &StrategyName,
    event: &RiskEvent,
) -> RuleDecision {
    if !matches!(
        event.kind,
        RiskKind::LiquidityRemoval | RiskKind::MempoolLiquidityRemoval
    ) {
        return RuleDecision::hold(RULE_NAME, "risk event is not liquidity removal");
    }

    let Some(pool_address) = event
        .pool_address
        .clone()
        .or_else(|| ctx.market.pool_address.clone())
    else {
        return RuleDecision::hold(RULE_NAME, "liquidity-removal event has no pool");
    };

    if !has_open_matching_position(ctx, strategy_name, &pool_address) {
        return RuleDecision::hold(RULE_NAME, "no open matching position");
    }

    RuleDecision::Exit {
        rule: exit_rule_name(event),
    }
}

fn exit_rule_name(event: &RiskEvent) -> &'static str {
    if event.kind == RiskKind::MempoolLiquidityRemoval
        || event.source.as_deref() == Some(RISK_SOURCE_MEMPOOL_SIGNAL)
    {
        MEMPOOL_SIGNAL_RULE_NAME
    } else {
        RULE_NAME
    }
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
