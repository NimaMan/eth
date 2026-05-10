use eth_alpha_core::market::PoolSnapshot;

use crate::snipe_all::{rule::RuleDecision, state::SnipeAllState};

pub const RULE_NAME: &str = "entry.buy_eligible_pool_once";

/// Strategy-specific entry rule: buy each pool at most once.
///
/// Eligibility (liquidity, denom, scam checks) is handled by the shared
/// `shared_rules::entry::eligibility` rule before this logic runs.
pub fn evaluate(state: &SnipeAllState, pool: &PoolSnapshot) -> RuleDecision {
    if state.has_bought(&pool.address) {
        return RuleDecision::hold(RULE_NAME, "pool already bought");
    }

    RuleDecision::Enter { rule: RULE_NAME }
}
