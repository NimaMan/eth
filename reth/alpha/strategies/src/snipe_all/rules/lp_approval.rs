use eth_alpha_core::{risk::RiskEvent, StrategyContext};

use crate::snipe_all::{config::SnipeAllConfig, rule::RuleDecision};

pub const RULE_NAME: &str = "exit.lp_approval";

pub fn evaluate(
    _config: &SnipeAllConfig,
    _ctx: &StrategyContext<'_>,
    _event: &RiskEvent,
) -> RuleDecision {
    RuleDecision::hold(RULE_NAME, "lp-approval exit labels are not enabled yet")
}
