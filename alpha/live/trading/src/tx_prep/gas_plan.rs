use eth_alpha_core::amount::DecimalAmount;
use serde::{Deserialize, Serialize};

use super::budget::{estimate_eth_cost_from_gwei, PriorityFeeBudget};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RankedFeeCandidate {
    pub label: String,
    pub priority_fee_gwei: DecimalAmount,
    pub max_fee_per_gas_gwei: DecimalAmount,
    pub rank_position_p50: Option<u64>,
    pub gas_before_p50: Option<u64>,
    pub likely_fits_at_p50: Option<bool>,
    #[serde(default)]
    pub source: Option<String>,
}

impl RankedFeeCandidate {
    pub fn estimated_priority_spend_eth(&self, gas_used: u64) -> DecimalAmount {
        estimate_eth_cost_from_gwei(self.priority_fee_gwei, gas_used)
    }

    pub fn estimated_max_cost_eth(&self, gas_used: u64) -> DecimalAmount {
        estimate_eth_cost_from_gwei(self.max_fee_per_gas_gwei, gas_used)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GasPlan {
    pub label: String,
    pub priority_fee_gwei: DecimalAmount,
    pub max_fee_per_gas_gwei: DecimalAmount,
    pub estimated_priority_spend_eth: DecimalAmount,
    pub estimated_max_cost_eth: DecimalAmount,
    pub rank_position_p50: Option<u64>,
    pub gas_before_p50: Option<u64>,
    pub likely_fits_at_p50: Option<bool>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GasPlanDecision {
    UseRanked(GasPlan),
    UseValueCap(GasPlan),
    Reject {
        reason: String,
        required_priority_fee_gwei: Option<DecimalAmount>,
        max_priority_fee_gwei: DecimalAmount,
        max_priority_spend_eth: DecimalAmount,
    },
}

pub fn choose_ranked_fee(
    budget: &PriorityFeeBudget,
    candidates: &[RankedFeeCandidate],
    allow_value_cap_fallback: bool,
) -> GasPlanDecision {
    let gas_used = budget.estimated_gas_used;
    let mut eligible = candidates
        .iter()
        .filter(|candidate| {
            budget.allows_priority_fee(candidate.priority_fee_gwei, gas_used)
                && budget.allows_total_fee(candidate.max_fee_per_gas_gwei, gas_used)
        })
        .collect::<Vec<_>>();

    eligible.sort_by(|left, right| compare_candidates(left, right));

    if let Some(candidate) = eligible.first() {
        return GasPlanDecision::UseRanked(plan_from_candidate(candidate, gas_used));
    }

    if allow_value_cap_fallback && budget.max_priority_fee_gwei > DecimalAmount::ZERO {
        let candidate = RankedFeeCandidate {
            label: "value_cap".to_string(),
            priority_fee_gwei: budget.max_priority_fee_gwei,
            max_fee_per_gas_gwei: budget.max_fee_per_gas_gwei,
            rank_position_p50: None,
            gas_before_p50: None,
            likely_fits_at_p50: None,
            source: Some("value_cap_budget".to_string()),
        };
        return GasPlanDecision::UseValueCap(plan_from_candidate(&candidate, gas_used));
    }

    let required_priority_fee_gwei = candidates
        .iter()
        .map(|candidate| candidate.priority_fee_gwei)
        .min();

    GasPlanDecision::Reject {
        reason: "gas_rank_exceeds_value_cap".to_string(),
        required_priority_fee_gwei,
        max_priority_fee_gwei: budget.max_priority_fee_gwei,
        max_priority_spend_eth: budget.max_priority_spend_eth,
    }
}

fn compare_candidates(left: &RankedFeeCandidate, right: &RankedFeeCandidate) -> std::cmp::Ordering {
    let left_rank = left.rank_position_p50.unwrap_or(u64::MAX);
    let right_rank = right.rank_position_p50.unwrap_or(u64::MAX);

    left_rank
        .cmp(&right_rank)
        .then_with(|| right.priority_fee_gwei.cmp(&left.priority_fee_gwei))
}

fn plan_from_candidate(candidate: &RankedFeeCandidate, gas_used: u64) -> GasPlan {
    GasPlan {
        label: candidate.label.clone(),
        priority_fee_gwei: candidate.priority_fee_gwei,
        max_fee_per_gas_gwei: candidate.max_fee_per_gas_gwei,
        estimated_priority_spend_eth: candidate.estimated_priority_spend_eth(gas_used),
        estimated_max_cost_eth: candidate.estimated_max_cost_eth(gas_used),
        rank_position_p50: candidate.rank_position_p50,
        gas_before_p50: candidate.gas_before_p50,
        likely_fits_at_p50: candidate.likely_fits_at_p50,
        source: candidate.source.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tx_prep::{PriorityFeeBudget, PriorityFeeBudgetInput};

    fn budget(max_priority_fee_gwei: i64) -> PriorityFeeBudget {
        PriorityFeeBudget::from_input(&PriorityFeeBudgetInput {
            protected_exit_value_eth: DecimalAmount::new(5, 2),
            expected_late_recovery_eth: DecimalAmount::ZERO,
            safety_buffer_eth: DecimalAmount::ZERO,
            predicted_base_fee_gwei: DecimalAmount::from(10),
            estimated_gas_used: 100_000,
            max_total_fee_eth: DecimalAmount::new(5, 2),
            configured_max_priority_fee_gwei: DecimalAmount::from(max_priority_fee_gwei),
        })
    }

    fn candidate(label: &str, priority_gwei: i64, rank: u64) -> RankedFeeCandidate {
        RankedFeeCandidate {
            label: label.to_string(),
            priority_fee_gwei: DecimalAmount::from(priority_gwei),
            max_fee_per_gas_gwei: DecimalAmount::from(priority_gwei + 10),
            rank_position_p50: Some(rank),
            gas_before_p50: Some(rank * 21_000),
            likely_fits_at_p50: Some(true),
            source: Some("test".to_string()),
        }
    }

    #[test]
    fn picks_best_rank_within_value_cap() {
        let decision = choose_ranked_fee(
            &budget(60),
            &[candidate("urgent", 100, 5), candidate("aggressive", 50, 10)],
            false,
        );

        match decision {
            GasPlanDecision::UseRanked(plan) => assert_eq!(plan.label, "aggressive"),
            other => panic!("expected eligible ranked plan, got {other:?}"),
        }
    }

    #[test]
    fn rejects_when_all_ranked_candidates_exceed_cap() {
        let decision = choose_ranked_fee(&budget(40), &[candidate("aggressive", 50, 10)], false);

        assert!(matches!(decision, GasPlanDecision::Reject { .. }));
    }

    #[test]
    fn can_fallback_to_value_cap_when_rank_quotes_are_too_expensive() {
        let decision = choose_ranked_fee(&budget(40), &[candidate("aggressive", 50, 10)], true);

        match decision {
            GasPlanDecision::UseValueCap(plan) => {
                assert_eq!(plan.label, "value_cap");
                assert_eq!(plan.priority_fee_gwei, DecimalAmount::from(40));
            }
            other => panic!("expected value-cap fallback, got {other:?}"),
        }
    }
}
