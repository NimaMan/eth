use eth_alpha_core::market::PoolSnapshot;

use crate::snipe_all::{config::SnipeAllConfig, rule::RuleDecision, state::SnipeAllState};

pub const RULE_NAME: &str = "entry.buy_eligible_pool_once";

pub fn evaluate(
    config: &SnipeAllConfig,
    state: &SnipeAllState,
    pool: &PoolSnapshot,
) -> RuleDecision {
    if state.has_bought(&pool.address) {
        return RuleDecision::hold(RULE_NAME, "pool already bought");
    }
    let decision = config.classification_decision(pool);
    if let Some(reason) = decision.reason {
        return RuleDecision::hold(RULE_NAME, reason.key());
    }
    if !decision.tradable_now {
        let reason = decision
            .eligible_outcome
            .map(|outcome| outcome.key())
            .unwrap_or_else(|| decision.category.key());
        return RuleDecision::hold(RULE_NAME, reason);
    }

    RuleDecision::Enter { rule: RULE_NAME }
}
