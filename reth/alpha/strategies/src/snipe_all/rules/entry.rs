use eth_alpha_core::market::PoolSnapshot;

use crate::snipe_all::{config::SnipeAllConfig, rule::RuleDecision, state::SnipeAllState};

pub const RULE_NAME: &str = "entry.buy_eligible_pool_once";

pub fn evaluate(
    config: &SnipeAllConfig,
    state: &SnipeAllState,
    pool: &PoolSnapshot,
) -> RuleDecision {
    if state.has_bought(pool.address) {
        return RuleDecision::hold(RULE_NAME, "pool already bought");
    }
    if !pool.can_buy {
        return RuleDecision::hold(RULE_NAME, "pool cannot be bought");
    }
    if !pool.can_sell {
        return RuleDecision::hold(RULE_NAME, "pool cannot be sold");
    }
    if pool.is_scam {
        return RuleDecision::hold(RULE_NAME, "pool is flagged as scam");
    }
    if pool.denom_reserve < config.min_denom_reserve {
        return RuleDecision::hold(RULE_NAME, "denom reserve below threshold");
    }

    RuleDecision::Enter { rule: RULE_NAME }
}
