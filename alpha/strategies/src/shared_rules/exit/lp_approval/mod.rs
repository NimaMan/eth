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

    if let Some(defer) =
        immediate_approval_deferral(ctx, event, defer_max_trading_enabled_age_blocks)
    {
        return RuleDecision::hold(
            RULE_NAME,
            format!(
                "early_approval_deferred_to_max_hold:age_blocks={} max_age_blocks={} age_basis={} reference_block={} observed_block={}",
                defer.age_blocks,
                defer.max_age_blocks,
                defer.basis,
                optional_block(defer.reference_block),
                optional_block(defer.observed_block),
            ),
        );
    }

    RuleDecision::Exit { rule: RULE_NAME }
}

struct ApprovalDeferral {
    age_blocks: i64,
    max_age_blocks: u64,
    basis: &'static str,
    reference_block: Option<u64>,
    observed_block: Option<u64>,
}

fn immediate_approval_deferral(
    ctx: &StrategyContext<'_>,
    event: &RiskEvent,
    max_age_blocks: Option<u64>,
) -> Option<ApprovalDeferral> {
    let Some(max_age_blocks) = max_age_blocks else {
        return None;
    };
    let Some(evidence) = approval_age_evidence(ctx, event) else {
        return None;
    };
    if evidence.age_blocks >= 0 && evidence.age_blocks <= max_age_blocks as i64 {
        Some(ApprovalDeferral {
            age_blocks: evidence.age_blocks,
            max_age_blocks,
            basis: evidence.basis,
            reference_block: evidence.reference_block,
            observed_block: evidence.observed_block,
        })
    } else {
        None
    }
}

fn approval_age_evidence(
    ctx: &StrategyContext<'_>,
    event: &RiskEvent,
) -> Option<crate::shared_rules::lp_approval::ApprovalAgeEvidence> {
    crate::shared_rules::lp_approval::approval_age_evidence(event)
        .or_else(|| approval_age_from_pool_creation(ctx, event))
}

fn approval_age_from_pool_creation(
    ctx: &StrategyContext<'_>,
    event: &RiskEvent,
) -> Option<crate::shared_rules::lp_approval::ApprovalAgeEvidence> {
    let observed_block = event.observed_block?;
    let creation_block = ctx.market.pool.as_ref()?.creation_block?;
    Some(crate::shared_rules::lp_approval::ApprovalAgeEvidence {
        age_blocks: observed_block as i64 - creation_block as i64,
        basis: "pool_creation_block",
        reference_block: Some(creation_block),
        observed_block: Some(observed_block),
    })
}

fn optional_block(block: Option<u64>) -> String {
    block
        .map(|block| block.to_string())
        .unwrap_or_else(|| "unknown".to_string())
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
