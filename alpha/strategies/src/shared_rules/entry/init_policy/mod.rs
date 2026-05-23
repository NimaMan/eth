//! Shared entry initialization policy.
//!
//! This rule runs after basic pool eligibility and before strategy-local buy
//! logic. It owns thresholds about already-active pools, such as pool age and
//! price/initial movement at entry.

mod config;
mod evidence;

pub use config::EntryInitPolicyConfig;
pub use evidence::EntryInitEvidence;

use crate::baseline::snipe_all::rule::RuleDecision;

pub const RULE_NAME: &str = "entry.init_policy";

pub fn evaluate(evidence: &EntryInitEvidence, config: &EntryInitPolicyConfig) -> RuleDecision {
    if config.require_creation_block && evidence.creation_block.is_none() {
        return RuleDecision::hold(RULE_NAME, "missing_creation_block");
    }

    if evidence
        .creation_block
        .map(|creation_block| creation_block > evidence.entry_block)
        .unwrap_or(false)
    {
        return RuleDecision::hold(RULE_NAME, "creation_block_after_entry");
    }

    if let Some(max_age_blocks) = config.max_age_blocks {
        let Some(entry_age_blocks) = evidence.entry_age_blocks else {
            return RuleDecision::hold(RULE_NAME, "missing_creation_block");
        };
        if entry_age_blocks > max_age_blocks {
            return RuleDecision::hold(RULE_NAME, "pool_age_gt_max");
        }
    }

    if let Some(max_ratio) = config.max_price_ratio_to_initial {
        let Some(price_ratio) = evidence.price_ratio_to_initial else {
            return if config.allow_missing_price_ratio {
                RuleDecision::Enter { rule: RULE_NAME }
            } else {
                RuleDecision::hold(RULE_NAME, "price_to_initial_ratio_missing")
            };
        };
        if price_ratio > max_ratio {
            return RuleDecision::hold(RULE_NAME, "price_to_initial_ratio_gt_max");
        }
    }

    RuleDecision::Enter { rule: RULE_NAME }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn evidence() -> EntryInitEvidence {
        EntryInitEvidence {
            entry_block: 120,
            creation_block: Some(100),
            entry_age_blocks: Some(20),
            price_ratio_to_initial: Some(Decimal::new(12, 1)),
            denom_reserve: Decimal::from(1),
            can_buy: true,
            can_sell: true,
        }
    }

    #[test]
    fn default_policy_allows_entry() {
        let decision = evaluate(&evidence(), &EntryInitPolicyConfig::default());

        assert_eq!(decision, RuleDecision::Enter { rule: RULE_NAME });
    }

    #[test]
    fn blocks_age_above_threshold() {
        let decision = evaluate(
            &evidence(),
            &EntryInitPolicyConfig {
                max_age_blocks: Some(10),
                ..EntryInitPolicyConfig::default()
            },
        );

        assert_eq!(decision, RuleDecision::hold(RULE_NAME, "pool_age_gt_max"));
    }

    #[test]
    fn blocks_missing_creation_when_required_or_needed_for_age() {
        let missing = EntryInitEvidence {
            creation_block: None,
            entry_age_blocks: None,
            ..evidence()
        };

        assert_eq!(
            evaluate(
                &missing,
                &EntryInitPolicyConfig {
                    require_creation_block: true,
                    ..EntryInitPolicyConfig::default()
                },
            ),
            RuleDecision::hold(RULE_NAME, "missing_creation_block")
        );
        assert_eq!(
            evaluate(
                &missing,
                &EntryInitPolicyConfig {
                    max_age_blocks: Some(30),
                    ..EntryInitPolicyConfig::default()
                },
            ),
            RuleDecision::hold(RULE_NAME, "missing_creation_block")
        );
    }

    #[test]
    fn blocks_price_ratio_above_threshold() {
        let decision = evaluate(
            &evidence(),
            &EntryInitPolicyConfig {
                max_price_ratio_to_initial: Some(Decimal::new(11, 1)),
                ..EntryInitPolicyConfig::default()
            },
        );

        assert_eq!(
            decision,
            RuleDecision::hold(RULE_NAME, "price_to_initial_ratio_gt_max")
        );
    }

    #[test]
    fn missing_price_ratio_is_configurable() {
        let missing = EntryInitEvidence {
            price_ratio_to_initial: None,
            ..evidence()
        };

        assert_eq!(
            evaluate(
                &missing,
                &EntryInitPolicyConfig {
                    max_price_ratio_to_initial: Some(Decimal::new(15, 1)),
                    ..EntryInitPolicyConfig::default()
                },
            ),
            RuleDecision::Enter { rule: RULE_NAME }
        );
        assert_eq!(
            evaluate(
                &missing,
                &EntryInitPolicyConfig {
                    max_price_ratio_to_initial: Some(Decimal::new(15, 1)),
                    allow_missing_price_ratio: false,
                    ..EntryInitPolicyConfig::default()
                },
            ),
            RuleDecision::hold(RULE_NAME, "price_to_initial_ratio_missing")
        );
    }
}
