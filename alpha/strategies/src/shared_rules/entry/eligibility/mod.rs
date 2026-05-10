use eth_alpha_core::market::PoolSnapshot;
use eth_pool_classification::{classify_pool_with_config, PoolClassificationConfig, PoolClassificationInput};
use rust_decimal::prelude::ToPrimitive;

use crate::baseline::snipe_all::rule::RuleDecision;

pub const RULE_NAME: &str = "entry.eligibility";

/// Evaluate whether a pool is eligible for entry.
///
/// This is the first gate any strategy should apply in `on_market_event`.
/// Pools that fail classification (insufficient liquidity, unsupported denom,
/// scam, cannot buy/sell, etc.) are rejected before strategy-specific logic
/// runs.
///
/// Returns `RuleDecision::Enter` when the pool is eligible and tradable now,
/// otherwise returns `RuleDecision::Hold` with the classification reason.
pub fn evaluate(pool: &PoolSnapshot, config: &PoolClassificationConfig) -> RuleDecision {
    let input = PoolClassificationInput::new(
        pool.denom_symbol.clone(),
        pool.denom_reserve.to_f64(),
        pool.can_buy,
        pool.can_sell,
        pool.is_scam,
    );

    let decision = classify_pool_with_config(&input, config);

    if !decision.eligible {
        let reason = decision
            .reason_key
            .unwrap_or_else(|| decision.category.key());
        return RuleDecision::hold(RULE_NAME, reason);
    }

    if !decision.tradable_now {
        let reason = decision
            .eligible_outcome
            .map(|o| o.key())
            .unwrap_or_else(|| decision.category.key());
        return RuleDecision::hold(RULE_NAME, reason);
    }

    RuleDecision::Enter { rule: RULE_NAME }
}
