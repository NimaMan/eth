use eth_alpha_core::{ids::StrategyName, risk::RiskEvent, StrategyContext};
use rust_decimal::Decimal;

use crate::baseline::snipe_all::rule::RuleDecision;

pub const RULE_NAME: &str = crate::shared_rules::exit::lp_approval::RULE_NAME;

/// Compatibility wrapper. Exit-specific LP approval policy lives in
/// `shared_rules::exit::lp_approval`; shared parsing remains in this module.
pub fn evaluate(
    ctx: &StrategyContext<'_>,
    strategy_name: &StrategyName,
    event: &RiskEvent,
    min_pct: Option<Decimal>,
) -> RuleDecision {
    crate::shared_rules::exit::lp_approval::evaluate(ctx, strategy_name, event, min_pct, None)
}
