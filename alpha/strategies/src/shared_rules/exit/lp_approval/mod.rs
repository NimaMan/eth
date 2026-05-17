use eth_alpha_core::{ids::StrategyName, risk::RiskEvent, StrategyContext};

use crate::baseline::snipe_all::rule::RuleDecision;

pub const RULE_NAME: &str = crate::shared_rules::lp_approval::exit_gate::RULE_NAME;

/// Exit when an `LpApproval` risk event targets a pool with an open position.
pub fn evaluate(
    ctx: &StrategyContext<'_>,
    strategy_name: &StrategyName,
    event: &RiskEvent,
) -> RuleDecision {
    crate::shared_rules::lp_approval::exit_gate::evaluate(ctx, strategy_name, event, None)
}
