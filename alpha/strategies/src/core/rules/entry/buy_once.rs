use eth_alpha_core::market::PoolSnapshot;

use crate::core::{rule::RuleDecision, state::EngineState};

pub const RULE_NAME: &str = "entry.buy_eligible_pool_once";

/// Core default entry rule: buy each eligible pool at most once.
///
/// Eligibility (liquidity, denom, scam checks) is handled by the shared
/// `core::rules::entry::eligibility` rule before this logic runs.
pub fn evaluate(state: &EngineState, pool: &PoolSnapshot) -> RuleDecision {
    if state.has_bought(&pool.address) {
        return RuleDecision::hold(RULE_NAME, "pool already bought");
    }

    RuleDecision::Enter { rule: RULE_NAME }
}
