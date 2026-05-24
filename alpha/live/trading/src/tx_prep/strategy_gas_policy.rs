use serde::{Deserialize, Serialize};

use super::{choose_ranked_fee, GasPlanDecision, PriorityFeeBudget, RankedFeeCandidate};
use crate::{LpSignalSource, PrioritySellPlan, SellUrgency};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GasRankProfile {
    MempoolRace,
    Normal,
    P50,
    P55,
    P60,
    P65,
    P70,
    P75,
    P77,
    P85,
    P88,
    P90,
    P92,
    P94,
    P95,
    P96,
    P97,
    P99,
}

impl GasRankProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::MempoolRace => "mempool_race",
            Self::Normal => "normal",
            Self::P50 => "p50",
            Self::P55 => "p55",
            Self::P60 => "p60",
            Self::P65 => "p65",
            Self::P70 => "p70",
            Self::P75 => "p75",
            Self::P77 => "p77",
            Self::P85 => "p85",
            Self::P88 => "p88",
            Self::P90 => "p90",
            Self::P92 => "p92",
            Self::P94 => "p94",
            Self::P95 => "p95",
            Self::P96 => "p96",
            Self::P97 => "p97",
            Self::P99 => "p99",
        }
    }

    pub fn matches_candidate(self, candidate: &RankedFeeCandidate) -> bool {
        normalized_label(&candidate.label) == self.label()
    }
}

/// Orders named gas-rank profiles for one strategy decision.
///
/// Candidate labels must normalize to one of the explicit profile names:
/// `normal` or an explicit fee-percentile profile such as `p50`, `p85`, or
/// `p95`. Unprofiled candidates are
/// intentionally ignored so live execution cannot silently fall back to an
/// old fixed/shadow gas value.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StrategyGasRankPolicy {
    pub allowed_profiles: Vec<GasRankProfile>,
    pub preference_order: Vec<GasRankProfile>,
}

impl Default for StrategyGasRankPolicy {
    fn default() -> Self {
        Self::p95_first()
    }
}

impl StrategyGasRankPolicy {
    pub fn normal_first() -> Self {
        Self::with_preference_order(vec![GasRankProfile::Normal])
    }

    pub fn p50_first() -> Self {
        Self::with_preference_order(vec![GasRankProfile::P50, GasRankProfile::Normal])
    }

    pub fn p75_first() -> Self {
        Self::with_preference_order(vec![
            GasRankProfile::P75,
            GasRankProfile::P50,
            GasRankProfile::Normal,
        ])
    }

    pub fn p85_first() -> Self {
        Self::with_preference_order(vec![
            GasRankProfile::P85,
            GasRankProfile::P75,
            GasRankProfile::P50,
            GasRankProfile::Normal,
        ])
    }

    pub fn p90_first() -> Self {
        Self::with_preference_order(vec![
            GasRankProfile::P90,
            GasRankProfile::P75,
            GasRankProfile::P50,
            GasRankProfile::Normal,
        ])
    }

    pub fn p95_first() -> Self {
        Self::with_preference_order(vec![
            GasRankProfile::P95,
            GasRankProfile::P90,
            GasRankProfile::P75,
            GasRankProfile::P50,
            GasRankProfile::Normal,
        ])
    }

    pub fn mempool_race_only() -> Self {
        Self::with_preference_order(vec![GasRankProfile::MempoolRace])
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

/// The live transaction class used to choose the default gas-rank ladder.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum StrategyTxKind {
    /// Entry buy before we hold inventory.
    EntryBuy,
    /// Value-capped exit tx built from an LP/risk signal.
    PrioritySell {
        urgency: SellUrgency,
        signal_source: LpSignalSource,
    },
}

/// Default gas-rank ladders for live strategy transactions.
///
/// The low-level route/tx builders only create calldata, value, and gas limit.
/// Exact simulation supplies gas-used evidence before fee candidate selection:
///
/// - Entry buy: `p85 -> p75 -> p50 -> normal`.
/// - Routine strategy exits: `p85 -> p75 -> p50 -> normal`.
/// - Mempool LP approval / liquidity-removal exit: `mempool_race`.
/// - LP approval exit, including mined approval and buy-confirm-block approval:
///   `p90 -> p75 -> p50 -> normal`.
///
/// The selected ladder is still value-capped by `PriorityFeeBudget`; if a
/// higher-rank profile is too expensive, the policy falls back to the next
/// named profile in the ladder.
pub struct StrategyGasRankDefaults;

impl StrategyGasRankDefaults {
    /// Default for live entry buys.
    pub fn entry_buy_policy() -> StrategyGasRankPolicy {
        StrategyGasRankPolicy::p85_first()
    }

    /// Default for live priority sells, derived from sell urgency/source.
    pub fn priority_sell_policy(plan: &PrioritySellPlan) -> StrategyGasRankPolicy {
        Self::policy_for(StrategyTxKind::PrioritySell {
            urgency: plan.urgency.clone(),
            signal_source: plan.signal_source.clone(),
        })
    }

    /// Resolve the strategy transaction kind into its default gas-rank ladder.
    pub fn policy_for(kind: StrategyTxKind) -> StrategyGasRankPolicy {
        match kind {
            StrategyTxKind::EntryBuy => StrategyGasRankPolicy::p85_first(),
            StrategyTxKind::PrioritySell {
                urgency: SellUrgency::NormalExit,
                ..
            } => StrategyGasRankPolicy::p85_first(),
            StrategyTxKind::PrioritySell {
                urgency: SellUrgency::MempoolPreMine,
                signal_source: LpSignalSource::MempoolLpApproval,
            } => StrategyGasRankPolicy::mempool_race_only(),
            StrategyTxKind::PrioritySell {
                urgency: SellUrgency::MempoolPreMine,
                ..
            } => StrategyGasRankPolicy::mempool_race_only(),
            StrategyTxKind::PrioritySell {
                urgency: SellUrgency::MinedApprovalRace,
                ..
            }
            | StrategyTxKind::PrioritySell {
                urgency: SellUrgency::BuyConfirmBlockApproval,
                ..
            } => StrategyGasRankPolicy::p90_first(),
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
            metadata: None,
        }
    }

    #[test]
    fn chooses_p95_when_affordable() {
        let decision = StrategyGasRankPolicy::p95_first().choose_ranked_fee(
            &budget(100),
            &[
                candidate("p50", 2, 25),
                candidate("p95", 9, 5),
                candidate("p90", 4, 10),
            ],
        );

        match decision {
            GasPlanDecision::UseRanked(plan) => assert_eq!(plan.label, "p95"),
            other => panic!("expected p95, got {other:?}"),
        }
    }

    #[test]
    fn falls_back_to_best_affordable_profile() {
        let decision = StrategyGasRankPolicy::p95_first().choose_ranked_fee(
            &budget(4),
            &[
                candidate("p50", 2, 25),
                candidate("p95", 9, 5),
                candidate("p90", 4, 10),
            ],
        );

        match decision {
            GasPlanDecision::UseRanked(plan) => assert_eq!(plan.label, "p90"),
            other => panic!("expected p90, got {other:?}"),
        }
    }

    #[test]
    fn rejects_when_no_allowed_profile_fits_budget() {
        let policy = StrategyGasRankPolicy {
            allowed_profiles: vec![GasRankProfile::P95, GasRankProfile::P90],
            preference_order: vec![GasRankProfile::P95, GasRankProfile::P90],
        };
        let decision = policy.choose_ranked_fee(
            &budget(3),
            &[
                candidate("p50", 2, 25),
                candidate("p95", 9, 5),
                candidate("p90", 4, 10),
            ],
        );

        assert!(matches!(decision, GasPlanDecision::Reject { .. }));
    }

    #[test]
    fn rejects_unprofiled_candidates() {
        let decision = StrategyGasRankPolicy::p95_first()
            .choose_ranked_fee(&budget(40), &[candidate("legacy_fixed", 40, 100)]);

        assert!(matches!(decision, GasPlanDecision::Reject { .. }));
    }

    #[test]
    fn entry_buy_default_is_p85_first() {
        let selected = StrategyGasRankDefaults::entry_buy_policy()
            .choose_candidate(&[
                candidate("p90", 4, 10),
                candidate("p85", 3, 15),
                candidate("p50", 2, 25),
            ])
            .expect("candidate");

        assert_eq!(selected.label, "p85");
    }

    #[test]
    fn normal_exit_default_is_p85_first() {
        let selected = StrategyGasRankDefaults::policy_for(StrategyTxKind::PrioritySell {
            urgency: SellUrgency::NormalExit,
            signal_source: LpSignalSource::StrategyExit,
        })
        .choose_candidate(&[candidate("p90", 4, 10), candidate("p85", 3, 15)])
        .expect("candidate");

        assert_eq!(selected.label, "p85");
    }

    #[test]
    fn lp_approval_priority_sell_default_is_p90_first() {
        let selected = StrategyGasRankDefaults::policy_for(StrategyTxKind::PrioritySell {
            urgency: SellUrgency::MinedApprovalRace,
            signal_source: LpSignalSource::MinedLpApproval,
        })
        .choose_candidate(&[candidate("p85", 3, 15), candidate("p90", 4, 10)])
        .expect("candidate");

        assert_eq!(selected.label, "p90");
    }

    #[test]
    fn mempool_priority_sell_default_is_mempool_race_only() {
        let selected = StrategyGasRankDefaults::policy_for(StrategyTxKind::PrioritySell {
            urgency: SellUrgency::MempoolPreMine,
            signal_source: LpSignalSource::MempoolLpApproval,
        })
        .choose_candidate(&[
            candidate("p50", 2, 25),
            candidate("p95", 9, 5),
            candidate("mempool_race", 3, 0),
        ])
        .expect("candidate");

        assert_eq!(selected.label, "mempool_race");
    }
}
