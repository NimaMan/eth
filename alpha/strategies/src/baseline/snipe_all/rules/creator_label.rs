use eth_alpha_core::{risk::RiskEvent, StrategyContext};

use crate::baseline::snipe_all::{config::SnipeAllConfig, rule::RuleDecision};

pub const RULE_NAME: &str = "label.creator_flow";

pub fn evaluate(
    _config: &SnipeAllConfig,
    _ctx: &StrategyContext<'_>,
    _event: &RiskEvent,
) -> RuleDecision {
    RuleDecision::hold(RULE_NAME, "creator-flow labels are not enabled yet")
}
