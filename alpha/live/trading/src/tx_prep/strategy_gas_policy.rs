use serde::{Deserialize, Serialize};

use super::{GasPlanDecision, PriorityFeeBudget, RankedFeeCandidate, choose_ranked_fee};
use crate::{LpSignalSource, PrioritySellPlan, SellUrgency};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GasRankProfile {
    Minimum,
    Balanced,
    Aggressive,
    Urgent,
}

impl GasRankProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::Minimum => "minimum",
            Self::Balanced => "balanced",
            Self::Aggressive => "aggressive",
            Self::Urgent => "urgent",
        }
    }

    pub fn matches_candidate(self, candidate: &RankedFeeCandidate) -> bool {
        normalized_label(&candidate.label) == self.label()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StrategyGasRankPolicy {
    pub allowed_profiles: Vec<GasRankProfile>,
    pub preference_order: Vec<GasRankProfile>,
}

impl Default for StrategyGasRankPolicy {
    fn default() -> Self {
        Self::urgent_first()
    }
}

impl StrategyGasRankPolicy {
    pub fn minimum_first() -> Self {
        Self::with_preference_order(vec![GasRankProfile::Minimum])
    }

    pub fn balanced_first() -> Self {
        Self::with_preference_order(vec![GasRankProfile::Balanced, GasRankProfile::Minimum])
    }

    pub fn aggressive_first() -> Self {
        Self::with_preference_order(vec![
            GasRankProfile::Aggressive,
            GasRankProfile::Balanced,
            GasRankProfile::Minimum,
        ])
    }

    pub fn urgent_first() -> Self {
        Self::with_preference_order(vec![
            GasRankProfile::Urgent,
            GasRankProfile::Aggressive,
            GasRankProfile::Balanced,
            GasRankProfile::Minimum,
        ])
    }

    pub fn with_preference_order(profiles: Vec<GasRankProfile>) -> Self {
        Self {
            allowed_profiles: profiles.clone(),
            preference_order: profiles,
        }
    }

    pub fn choose_candidate(
        &self,
        candidates: &[RankedFeeCandidate],
    ) -> Option<RankedFeeCandidate> {
        self.ordered_candidates(candidates)
            .into_iter()
            .next()
            .cloned()
    }

    pub fn choose_ranked_fee(
        &self,
        budget: &PriorityFeeBudget,
        candidates: &[RankedFeeCandidate],
    ) -> GasPlanDecision {
        let ordered = self.ordered_candidates(candidates);
        for candidate in &ordered {
            let candidate = (*candidate).clone();
            let one = [candidate];
            if let GasPlanDecision::UseRanked(plan) = choose_ranked_fee(budget, &one) {
                return GasPlanDecision::UseRanked(plan);
            }
        }

        let reason = if ordered.is_empty() {
            "gas_rank_exceeds_strategy_policy"
        } else {
            "gas_rank_exceeds_value_cap"
        };
        GasPlanDecision::Reject {
            reason: reason.to_string(),
            required_priority_fee_gwei: ordered
                .iter()
                .map(|candidate| candidate.priority_fee_gwei)
                .min(),
            max_priority_fee_gwei: budget.max_priority_fee_gwei,
            max_priority_spend_eth: budget.max_priority_spend_eth,
        }
    }

    fn ordered_candidates<'a>(
        &self,
        candidates: &'a [RankedFeeCandidate],
    ) -> Vec<&'a RankedFeeCandidate> {
        let mut ordered = Vec::new();
        let mut used = vec![false; candidates.len()];

        for profile in self.preference_order.iter().copied() {
            if !self.allowed_profiles.contains(&profile) {
                continue;
            }
            for (index, candidate) in candidates.iter().enumerate() {
                if !used[index] && profile.matches_candidate(candidate) {
                    ordered.push(candidate);
                    used[index] = true;
                }
            }
        }

        for profile in self.allowed_profiles.iter().copied() {
            if self.preference_order.contains(&profile) {
                continue;
            }
            for (index, candidate) in candidates.iter().enumerate() {
                if !used[index] && profile.matches_candidate(candidate) {
                    ordered.push(candidate);
                    used[index] = true;
                }
            }
        }

        ordered
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum StrategyTxKind {
    EntryBuy,
    PrioritySell {
        urgency: SellUrgency,
        signal_source: LpSignalSource,
    },
}

pub struct StrategyGasRankDefaults;

impl StrategyGasRankDefaults {
    pub fn entry_buy_policy() -> StrategyGasRankPolicy {
        StrategyGasRankPolicy::aggressive_first()
    }

    pub fn priority_sell_policy(plan: &PrioritySellPlan) -> StrategyGasRankPolicy {
        Self::policy_for(StrategyTxKind::PrioritySell {
            urgency: plan.urgency.clone(),
            signal_source: plan.signal_source.clone(),
        })
    }

    pub fn policy_for(kind: StrategyTxKind) -> StrategyGasRankPolicy {
        match kind {
            StrategyTxKind::EntryBuy => StrategyGasRankPolicy::aggressive_first(),
            StrategyTxKind::PrioritySell {
                urgency: SellUrgency::MempoolPreMine,
                signal_source: LpSignalSource::MempoolLpApproval,
            } => StrategyGasRankPolicy::urgent_first(),
            StrategyTxKind::PrioritySell {
                urgency: SellUrgency::MempoolPreMine,
                ..
            } => StrategyGasRankPolicy::urgent_first(),
            StrategyTxKind::PrioritySell {
                urgency: SellUrgency::MinedApprovalRace,
                ..
            } => StrategyGasRankPolicy::urgent_first(),
            StrategyTxKind::PrioritySell {
                urgency: SellUrgency::BuyConfirmBlockApproval,
                ..
            } => StrategyGasRankPolicy::aggressive_first(),
        }
    }
}

fn normalized_label(label: &str) -> String {
    label.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tx_prep::{PriorityFeeBudget, PriorityFeeBudgetInput};
    use eth_alpha_core::amount::DecimalAmount;

    fn budget(max_priority_fee_gwei: i64) -> PriorityFeeBudget {
        PriorityFeeBudget::from_input(&PriorityFeeBudgetInput {
            protected_exit_value_eth: DecimalAmount::new(5, 2),
            expected_late_recovery_eth: DecimalAmount::ZERO,
            safety_buffer_eth: DecimalAmount::ZERO,
            predicted_base_fee_gwei: DecimalAmount::from(1),
            estimated_gas_used: 100_000,
            max_total_fee_eth: DecimalAmount::new(5, 2),
            configured_max_priority_fee_gwei: DecimalAmount::from(max_priority_fee_gwei),
        })
    }

    fn candidate(label: &str, priority_gwei: i64, rank: u64) -> RankedFeeCandidate {
        RankedFeeCandidate {
            label: label.to_string(),
            priority_fee_gwei: DecimalAmount::from(priority_gwei),
            max_fee_per_gas_gwei: DecimalAmount::from(priority_gwei + 1),
            rank_position_p50: Some(rank),
            gas_before_p50: Some(rank * 21_000),
            likely_fits_at_p50: Some(true),
            source: Some("test".to_string()),
        }
    }

    #[test]
    fn chooses_urgent_when_affordable() {
        let decision = StrategyGasRankPolicy::urgent_first().choose_ranked_fee(
            &budget(100),
            &[
                candidate("balanced", 2, 25),
                candidate("urgent", 9, 5),
                candidate("aggressive", 4, 10),
            ],
        );

        match decision {
            GasPlanDecision::UseRanked(plan) => assert_eq!(plan.label, "urgent"),
            other => panic!("expected urgent, got {other:?}"),
        }
    }

    #[test]
    fn falls_back_to_best_affordable_profile() {
        let decision = StrategyGasRankPolicy::urgent_first().choose_ranked_fee(
            &budget(4),
            &[
                candidate("balanced", 2, 25),
                candidate("urgent", 9, 5),
                candidate("aggressive", 4, 10),
            ],
        );

        match decision {
            GasPlanDecision::UseRanked(plan) => assert_eq!(plan.label, "aggressive"),
            other => panic!("expected aggressive, got {other:?}"),
        }
    }

    #[test]
    fn rejects_when_no_allowed_profile_fits_budget() {
        let policy = StrategyGasRankPolicy {
            allowed_profiles: vec![GasRankProfile::Urgent, GasRankProfile::Aggressive],
            preference_order: vec![GasRankProfile::Urgent, GasRankProfile::Aggressive],
        };
        let decision = policy.choose_ranked_fee(
            &budget(3),
            &[
                candidate("balanced", 2, 25),
                candidate("urgent", 9, 5),
                candidate("aggressive", 4, 10),
            ],
        );

        assert!(matches!(decision, GasPlanDecision::Reject { .. }));
    }

    #[test]
    fn rejects_unprofiled_candidates() {
        let decision = StrategyGasRankPolicy::urgent_first()
            .choose_ranked_fee(&budget(40), &[candidate("legacy_fixed", 40, 100)]);

        assert!(matches!(decision, GasPlanDecision::Reject { .. }));
    }

    #[test]
    fn entry_buy_default_is_aggressive_first() {
        let selected = StrategyGasRankDefaults::entry_buy_policy()
            .choose_candidate(&[candidate("urgent", 9, 5), candidate("aggressive", 4, 10)])
            .expect("candidate");

        assert_eq!(selected.label, "aggressive");
    }

    #[test]
    fn mempool_priority_sell_default_is_urgent_first() {
        let selected = StrategyGasRankDefaults::policy_for(StrategyTxKind::PrioritySell {
            urgency: SellUrgency::MempoolPreMine,
            signal_source: LpSignalSource::MempoolLpApproval,
        })
        .choose_candidate(&[candidate("balanced", 2, 25), candidate("urgent", 9, 5)])
        .expect("candidate");

        assert_eq!(selected.label, "urgent");
    }
}
