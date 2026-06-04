use eth_alpha_core::{
    ids::{PoolAddress, TokenAddress},
    StrategyContext,
};
use rust_decimal::Decimal;

use crate::core::rule::RuleDecision;

pub const RULE_NAME: &str = "entry.lp_approval_gate";

pub fn evaluate(
    ctx: &StrategyContext<'_>,
    token_address: TokenAddress,
    pool_address: &PoolAddress,
    min_pct: Option<Decimal>,
) -> RuleDecision {
    for event in ctx
        .active_risks
        .iter()
        .rev()
        .filter(|event| super::matches_token_pool(event, token_address, pool_address))
    {
        if super::approval_exceeds_threshold(event, min_pct) {
            return RuleDecision::hold(RULE_NAME, hold_reason(min_pct));
        }
    }

    RuleDecision::Enter { rule: RULE_NAME }
}

fn hold_reason(min_pct: Option<Decimal>) -> &'static str {
    if min_pct.is_some() {
        "approved_pct_gt_min"
    } else {
        "lp_approval_seen"
    }
}
