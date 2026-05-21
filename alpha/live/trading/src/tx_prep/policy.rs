use eth_alpha_core::amount::DecimalAmount;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{
    build_priority_sell_request, GasPlanDecision, PreSubmitSimulation, PreparedSellRoute,
    PriorityFeeBudget, PriorityFeeBudgetInput, RankedFeeCandidate, StrategyGasRankPolicy,
    TxPrepRequestContext, TxPrepRouteError, TxPrepSimulationError,
};
use crate::LiveTraderTxSignal;
use crate::PrioritySellPlan;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TxPrepConfig {
    pub max_total_fee_eth: DecimalAmount,
    pub max_priority_fee_gwei: DecimalAmount,
    pub safety_buffer_eth: DecimalAmount,
    pub gas_rank_policy: StrategyGasRankPolicy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrioritySellTxPrep {
    pub context: TxPrepRequestContext,
    pub plan: PrioritySellPlan,
    pub route: PreparedSellRoute,
    pub simulation: PreSubmitSimulation,
    pub predicted_base_fee_gwei: DecimalAmount,
    pub expected_late_recovery_eth: DecimalAmount,
    pub ranked_fee_candidates: Vec<RankedFeeCandidate>,
    #[serde(default)]
    pub gas_rank_policy: Option<StrategyGasRankPolicy>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TxPrepOutcome {
    Submit {
        signal: LiveTraderTxSignal,
        budget: PriorityFeeBudget,
    },
    Reject(TxPrepReject),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TxPrepReject {
    pub reason: String,
    pub metadata: Value,
}

pub fn prepare_priority_sell(config: &TxPrepConfig, input: PrioritySellTxPrep) -> TxPrepOutcome {
    if let Err(error) = input.route.validate() {
        return reject_route(error);
    }
    if let Err(error) = input.simulation.validate() {
        return reject_simulation(error);
    }

    let budget = PriorityFeeBudget::from_input(&PriorityFeeBudgetInput {
        protected_exit_value_eth: input.simulation.expected_recovery_eth,
        expected_late_recovery_eth: input.expected_late_recovery_eth,
        safety_buffer_eth: config.safety_buffer_eth,
        predicted_base_fee_gwei: input.predicted_base_fee_gwei,
        estimated_gas_used: input.route.estimated_gas_used,
        max_total_fee_eth: config.max_total_fee_eth.min(input.plan.max_total_fee_eth),
        configured_max_priority_fee_gwei: config
            .max_priority_fee_gwei
            .min(input.plan.max_priority_fee_per_gas_gwei),
    });

    let gas_rank_policy = input
        .gas_rank_policy
        .clone()
        .unwrap_or_else(|| config.gas_rank_policy.clone());

    match gas_rank_policy.choose_ranked_fee(&budget, &input.ranked_fee_candidates) {
        GasPlanDecision::UseRanked(gas_plan) => {
            let signal = build_priority_sell_request(
                &input.context,
                &input.plan,
                &input.route,
                &input.simulation,
                &budget,
                &gas_plan,
                &gas_rank_policy,
            );
            TxPrepOutcome::Submit { signal, budget }
        }
        GasPlanDecision::Reject {
            reason,
            required_priority_fee_gwei,
            max_priority_fee_gwei,
            max_priority_spend_eth,
        } => TxPrepOutcome::Reject(TxPrepReject {
            reason,
            metadata: json!({
                "required_priority_fee_gwei": required_priority_fee_gwei,
                "max_priority_fee_gwei": max_priority_fee_gwei,
                "max_priority_spend_eth": max_priority_spend_eth,
                "ranked_fee_candidates": input.ranked_fee_candidates,
                "strategy_gas_rank_policy": gas_rank_policy,
                "budget": budget,
            }),
        }),
    }
}

fn reject_route(error: TxPrepRouteError) -> TxPrepOutcome {
    TxPrepOutcome::Reject(TxPrepReject {
        reason: "invalid_prepared_route".to_string(),
        metadata: json!({ "error": error.to_string() }),
    })
}

fn reject_simulation(error: TxPrepSimulationError) -> TxPrepOutcome {
    TxPrepOutcome::Reject(TxPrepReject {
        reason: "invalid_pre_submit_simulation".to_string(),
        metadata: json!({ "error": error.to_string() }),
    })
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use eth_alpha_core::decision_rationale::source;
    use eth_alpha_core::ids::{PoolAddress, TradeId};

    use super::*;
    use crate::{LpSignalSource, PriorityRoute, SellUrgency};

    fn plan() -> PrioritySellPlan {
        PrioritySellPlan {
            trade_id: TradeId("trd_test".to_string()),
            token_address: Address::with_last_byte(0x11),
            pool_address: PoolAddress::from("0xtoken:0xpool"),
            observed_block: 25_128_246,
            signal_source: LpSignalSource::MempoolLpApproval,
            urgency: SellUrgency::MempoolPreMine,
            route: PriorityRoute::PrivateRelay,
            max_priority_fee_per_gas_gwei: DecimalAmount::from(100),
            max_total_fee_eth: DecimalAmount::new(2, 2),
            reason: "exit.mempool_liquidity_removal_signal".to_string(),
        }
    }

    fn input(candidate_priority_gwei: i64) -> PrioritySellTxPrep {
        PrioritySellTxPrep {
            context: TxPrepRequestContext {
                chain_id: 1,
                from: "0x0000000000000000000000000000000000000001".to_string(),
                strategy_name: "alpha11-live-univ2-lp30-pool-update-block-hold20".to_string(),
                strategy_run_id: Some("run-1".to_string()),
                observed_block: Some(25_128_246),
                source_metadata: json!({ "signal_id": 222 }),
            },
            plan: plan(),
            route: PreparedSellRoute {
                protocol: "UniswapV2".to_string(),
                router_address: "0x0000000000000000000000000000000000000002".to_string(),
                calldata: "0x1234".to_string(),
                value_wei: "0".to_string(),
                gas_limit: 180_000,
                estimated_gas_used: 150_000,
                max_slippage_bps: Some(500),
            },
            simulation: PreSubmitSimulation {
                block_number: 25_128_246,
                block_hash: Some("0xabc".to_string()),
                state_root: None,
                expected_output_token: Some("WETH".to_string()),
                expected_output_amount: Some("10000000000000000".to_string()),
                min_output_amount: Some("9000000000000000".to_string()),
                expected_recovery_eth: DecimalAmount::new(1, 2),
                gas_used: Some(150_000),
                would_revert: false,
                metadata: json!({ "sim": "ok" }),
            },
            predicted_base_fee_gwei: DecimalAmount::from(10),
            expected_late_recovery_eth: DecimalAmount::new(6, 4),
            ranked_fee_candidates: vec![RankedFeeCandidate {
                label: "aggressive".to_string(),
                priority_fee_gwei: DecimalAmount::from(candidate_priority_gwei),
                max_fee_per_gas_gwei: DecimalAmount::from(candidate_priority_gwei + 10),
                rank_position_p50: Some(10),
                gas_before_p50: Some(450_000),
                likely_fits_at_p50: Some(true),
                source: Some("gas_rank".to_string()),
            }],
            gas_rank_policy: None,
        }
    }

    fn config(max_priority_gwei: i64) -> TxPrepConfig {
        TxPrepConfig {
            max_total_fee_eth: DecimalAmount::new(2, 2),
            max_priority_fee_gwei: DecimalAmount::from(max_priority_gwei),
            safety_buffer_eth: DecimalAmount::new(1, 3),
            gas_rank_policy: StrategyGasRankPolicy::urgent_first(),
        }
    }

    #[test]
    fn prepares_kartal_request_with_value_capped_bribe() {
        let outcome = prepare_priority_sell(&config(100), input(40));

        match outcome {
            TxPrepOutcome::Submit { signal, budget } => {
                assert_eq!(signal.trade_id.as_ref().unwrap().0, "trd_test");
                assert_eq!(signal.request.max_priority_fee_per_gas, "40000000000");
                assert_eq!(
                    signal.request.bribe.as_ref().unwrap().priority_fee_per_gas,
                    "40000000000"
                );
                assert_eq!(signal.request.max_fee_per_gas, "50000000000");
                assert_eq!(
                    signal.request.metadata["wire_protocol"],
                    json!("eth_direct_raw_v1")
                );
                assert_eq!(
                    signal.request.metadata["reason_code"],
                    json!("exit.mempool_liquidity_removal_signal")
                );
                assert_eq!(signal.request.metadata["reason_category"], json!("exit"));
                assert_eq!(
                    signal.request.metadata["reason_source"],
                    json!(source::EVENT_SOURCE_MEMPOOL_SIGNAL)
                );
                assert_eq!(
                    signal.request.metadata["decision_reason"]["code"],
                    json!("exit.mempool_liquidity_removal_signal")
                );
                assert_eq!(
                    signal.request.metadata["signal_source"],
                    json!("MempoolLpApproval")
                );
                assert_eq!(signal.request.metadata["source"]["signal_id"], json!(222));
                assert!(budget.max_priority_fee_gwei >= DecimalAmount::from(40));
            }
            other => panic!("expected submit, got {other:?}"),
        }
    }

    #[test]
    fn rejects_when_required_bribe_exceeds_value_cap() {
        let outcome = prepare_priority_sell(&config(10), input(50));

        match outcome {
            TxPrepOutcome::Reject(reject) => {
                assert_eq!(reject.reason, "gas_rank_exceeds_value_cap");
            }
            other => panic!("expected reject, got {other:?}"),
        }
    }

    #[test]
    fn rejects_reverting_simulation_before_request_build() {
        let mut input = input(50);
        input.simulation.would_revert = true;

        let outcome = prepare_priority_sell(&config(100), input);

        match outcome {
            TxPrepOutcome::Reject(reject) => {
                assert_eq!(reject.reason, "invalid_pre_submit_simulation");
            }
            other => panic!("expected reject, got {other:?}"),
        }
    }
}
