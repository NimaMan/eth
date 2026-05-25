use eth_alpha_core::ids::BlockNumber;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    gas_plan::GasPlan, gwei_to_wei_string, metadata::tx_prep_metadata, PreSubmitSimulation,
    PreparedSellRoute, PriorityFeeBudget, StrategyGasRankPolicy,
};
use crate::{
    KartalBribeRequest, KartalSimulationReference, LiveDirectRawTransactionRequest,
    LiveTraderTxSignal, PrioritySellPlan,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TxPrepRequestContext {
    pub chain_id: u64,
    pub from: String,
    pub strategy_name: String,
    pub strategy_run_id: Option<String>,
    pub observed_block: Option<BlockNumber>,
    pub required_state_block: BlockNumber,
    #[serde(default)]
    pub source_metadata: Value,
}

pub fn build_priority_sell_request(
    context: &TxPrepRequestContext,
    plan: &PrioritySellPlan,
    route: &PreparedSellRoute,
    simulation: &PreSubmitSimulation,
    budget: &PriorityFeeBudget,
    gas_plan: &GasPlan,
    gas_rank_policy: &StrategyGasRankPolicy,
) -> LiveTraderTxSignal {
    let mut metadata = tx_prep_metadata(
        plan,
        route,
        simulation,
        budget,
        gas_plan,
        gas_rank_policy,
        context.source_metadata.clone(),
    );
    if let Some(map) = metadata.as_object_mut() {
        map.insert(
            "state_dependency".to_string(),
            serde_json::json!({
                "decision_block": context.observed_block.or(Some(plan.observed_block)),
                "signal_observed_block": context.observed_block,
                "required_state_block": context.required_state_block,
                "simulation_block": simulation.block_number,
            }),
        );
    }

    let request = LiveDirectRawTransactionRequest {
        attempt_id: Some(format!(
            "{}-priority-exit-{}",
            plan.trade_id.0, plan.observed_block
        )),
        chain_id: context.chain_id,
        from: context.from.clone(),
        to: route.router_address.clone(),
        value: route.value_wei.clone(),
        data: route.calldata.clone(),
        gas_limit: route.gas_limit.to_string(),
        max_fee_per_gas: gwei_to_wei_string(gas_plan.max_fee_per_gas_gwei),
        max_priority_fee_per_gas: gwei_to_wei_string(gas_plan.priority_fee_gwei),
        nonce: None,
        bribe: Some(KartalBribeRequest {
            priority_fee_per_gas: gwei_to_wei_string(gas_plan.priority_fee_gwei),
            max_fee_per_gas: Some(gwei_to_wei_string(gas_plan.max_fee_per_gas_gwei)),
        }),
        simulation: Some(KartalSimulationReference {
            block_number: simulation.block_number,
            block_hash: simulation.block_hash.clone(),
            state_root: simulation.state_root.clone(),
            expected_output_token: simulation.expected_output_token.clone(),
            expected_output_amount: simulation.expected_output_amount.clone(),
            min_output_amount: simulation.min_output_amount.clone(),
            metadata: simulation.metadata(),
        }),
        metadata,
    };

    LiveTraderTxSignal {
        strategy_name: context.strategy_name.clone(),
        strategy_run_id: context.strategy_run_id.clone(),
        trade_id: Some(plan.trade_id.clone()),
        token_address: Some(plan.token_address),
        pool_address: Some(plan.pool_address.clone()),
        observed_block: context.observed_block.or(Some(plan.observed_block)),
        submission_policy: crate::TxSubmissionPolicy::PublicMempool,
        request,
    }
}
