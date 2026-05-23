use eth_alpha_core::{
    ids::{PoolAddress, StrategyName},
    risk::{RiskEvent, RiskKind},
    StrategyContext,
};
use rust_decimal::Decimal;

use crate::baseline::snipe_all::rule::RuleDecision;

pub const RULE_NAME: &str = "exit.lp_approval";
pub const DEFAULT_DEFER_MAX_TRADING_ENABLED_AGE_BLOCKS: u64 = 2;

/// Exit when an `LpApproval` risk event targets a pool with an open position.
pub fn evaluate(
    ctx: &StrategyContext<'_>,
    strategy_name: &StrategyName,
    event: &RiskEvent,
    min_pct: Option<Decimal>,
    defer_max_trading_enabled_age_blocks: Option<u64>,
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

    if !crate::shared_rules::lp_approval::approval_exceeds_threshold(event, min_pct) {
        return RuleDecision::hold(RULE_NAME, "approved_pct_unknown_or_not_gt_min");
    }

    if should_defer_immediate_approval(ctx, event, defer_max_trading_enabled_age_blocks) {
        return RuleDecision::hold(RULE_NAME, "early_approval_deferred_to_max_hold");
    }

    RuleDecision::Exit { rule: RULE_NAME }
}

fn should_defer_immediate_approval(
    ctx: &StrategyContext<'_>,
    event: &RiskEvent,
    max_age_blocks: Option<u64>,
) -> bool {
    let Some(max_age_blocks) = max_age_blocks else {
        return false;
    };
    let Some(age_blocks) = approval_trading_enabled_age_blocks(ctx, event) else {
        return false;
    };
    age_blocks >= 0 && age_blocks <= max_age_blocks as i64
}

fn approval_trading_enabled_age_blocks(
    ctx: &StrategyContext<'_>,
    event: &RiskEvent,
) -> Option<i64> {
    crate::shared_rules::lp_approval::approval_trading_enabled_age_blocks(event)
        .or_else(|| approval_age_from_pool_creation(ctx, event))
}

fn approval_age_from_pool_creation(ctx: &StrategyContext<'_>, event: &RiskEvent) -> Option<i64> {
    let observed_block = event.observed_block?;
    let creation_block = ctx.market.pool.as_ref()?.creation_block?;
    Some(observed_block as i64 - creation_block as i64)
}

fn has_open_matching_position(
    ctx: &StrategyContext<'_>,
    strategy_name: &StrategyName,
    pool_address: &PoolAddress,
) -> bool {
    ctx.portfolio.positions.values().any(|position| {
        &position.key.pool_address == pool_address
            && position.key.strategy_name == *strategy_name
            && position.can_submit_exit()
    })
}
