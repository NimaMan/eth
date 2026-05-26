use eth_alpha_core::amount::DecimalAmount;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::budget::{estimate_eth_cost_from_gwei, PriorityFeeBudget};

pub const MEMPOOL_RACE_GAS_LABEL: &str = "mempool_race";
pub const MEMPOOL_RACE_GAS_SOURCE: &str = "eth_public_mempool_pending_tx_fee";

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GasPlanDecision {
    UseRanked(GasPlan),
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

pub fn apply_min_priority_fee_floor_to_candidates(
    candidates: impl IntoIterator<Item = RankedFeeCandidate>,
    min_priority_fee_gwei: DecimalAmount,
) -> Vec<RankedFeeCandidate> {
    candidates
        .into_iter()
        .map(|candidate| apply_min_priority_fee_floor(candidate, min_priority_fee_gwei))
        .collect()
}

pub fn apply_min_priority_fee_floor(
    mut candidate: RankedFeeCandidate,
    min_priority_fee_gwei: DecimalAmount,
) -> RankedFeeCandidate {
    if min_priority_fee_gwei <= DecimalAmount::ZERO
        || candidate.priority_fee_gwei >= min_priority_fee_gwei
    {
        return candidate;
    }

    let original_priority_fee_gwei = candidate.priority_fee_gwei;
    let original_max_fee_per_gas_gwei = candidate.max_fee_per_gas_gwei;
    let priority_delta_gwei = min_priority_fee_gwei - original_priority_fee_gwei;

    candidate.priority_fee_gwei = min_priority_fee_gwei;
    candidate.max_fee_per_gas_gwei += priority_delta_gwei;
    candidate.metadata = Some(merge_floor_metadata(
        candidate.metadata.take(),
        json!({
            "strategy_min_priority_fee_floor_applied": true,
            "strategy_min_priority_fee_gwei": min_priority_fee_gwei,
            "original_priority_fee_gwei": original_priority_fee_gwei,
            "original_max_fee_per_gas_gwei": original_max_fee_per_gas_gwei,
        }),
    ));
    candidate
}

fn merge_floor_metadata(existing: Option<Value>, floor_metadata: Value) -> Value {
    let mut map = match existing {
        Some(Value::Object(map)) => map,
        Some(value) => {
            let mut map = Map::new();
            map.insert("original_metadata".to_string(), value);
            map
        }
        None => Map::new(),
    };

    if let Value::Object(floor_metadata) = floor_metadata {
        for (key, value) in floor_metadata {
            map.insert(key, value);
        }
    }

    Value::Object(map)
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
        metadata: candidate.metadata.clone(),
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
            metadata: None,
        }
    }

    #[test]
    fn picks_best_rank_within_value_cap() {
        let decision = choose_ranked_fee(
            &budget(60),
            &[candidate("p95", 100, 5), candidate("p90", 50, 10)],
        );

        match decision {
            GasPlanDecision::UseRanked(plan) => assert_eq!(plan.label, "p90"),
            other => panic!("expected eligible ranked plan, got {other:?}"),
        }
    }

    #[test]
    fn rejects_when_all_ranked_candidates_exceed_cap() {
        let decision = choose_ranked_fee(&budget(40), &[candidate("p90", 50, 10)]);

        assert!(matches!(decision, GasPlanDecision::Reject { .. }));
    }

    #[test]
    fn applies_strategy_min_priority_fee_floor_preserving_base_fee_cushion() {
        let candidate = RankedFeeCandidate {
            label: "p85".to_string(),
            priority_fee_gwei: DecimalAmount::new(5, 1),
            max_fee_per_gas_gwei: DecimalAmount::new(6, 1),
            rank_position_p50: Some(10),
            gas_before_p50: Some(210_000),
            likely_fits_at_p50: Some(true),
            source: Some("test".to_string()),
            metadata: Some(json!({ "existing": true })),
        };

        let floored = apply_min_priority_fee_floor(candidate, DecimalAmount::ONE);

        assert_eq!(floored.priority_fee_gwei, DecimalAmount::ONE);
        assert_eq!(floored.max_fee_per_gas_gwei, DecimalAmount::new(11, 1));
        let metadata = floored.metadata.expect("floor metadata");
        assert_eq!(
            metadata["strategy_min_priority_fee_floor_applied"],
            json!(true)
        );
        assert_eq!(metadata["existing"], json!(true));
        assert_eq!(
            metadata["original_priority_fee_gwei"],
            json!(DecimalAmount::new(5, 1))
        );
    }
}
