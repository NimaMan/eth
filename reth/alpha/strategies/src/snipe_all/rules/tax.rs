use eth_alpha_core::{risk::RiskEvent, StrategyContext};

use crate::snipe_all::{config::SnipeAllConfig, rule::RuleDecision};

pub const RULE_NAME: &str = "exit.tax";

pub fn evaluate(
    _config: &SnipeAllConfig,
    _ctx: &StrategyContext<'_>,
    _event: &RiskEvent,
) -> RuleDecision {
    RuleDecision::hold(RULE_NAME, "tax exits are not enabled yet")
}
