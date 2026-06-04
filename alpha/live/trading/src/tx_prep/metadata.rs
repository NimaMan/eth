use serde_json::{json, Map, Value};

use super::{
    GasPlan, PreSubmitSimulation, PreparedSellRoute, PriorityFeeBudget, StrategyGasRankPolicy,
};
use crate::{LpSignalSource, PrioritySellPlan, SellUrgency, ETH_UNSIGNED_TX_WIRE_PROTOCOL};

pub fn tx_prep_metadata(
    plan: &PrioritySellPlan,
    route: &PreparedSellRoute,
    simulation: &PreSubmitSimulation,
    budget: &PriorityFeeBudget,
    gas_plan: &GasPlan,
    gas_rank_policy: &StrategyGasRankPolicy,
    source: Value,
) -> Value {
    let mut map = Map::new();
    let decision_reason = plan.decision_reason();

    map.insert(
        "wire_protocol".to_string(),
        json!(ETH_UNSIGNED_TX_WIRE_PROTOCOL),
    );
    map.insert("intent_kind".to_string(), json!("priority_sell"));
    map.insert("executor_boundary".to_string(), json!("eth_tx_executor"));
    map.insert("tx_prep_version".to_string(), json!(1));
    map.insert("reason".to_string(), json!(plan.reason));
    map.insert("signal_source".to_string(), json!(plan.signal_source));
    if let Some(reason) = &decision_reason {
        map.insert("reason_code".to_string(), json!(reason.code));
        map.insert("reason_category".to_string(), json!(reason.category_key()));
        map.insert("reason_label".to_string(), json!(reason.label));
        if let Some(source) = &reason.source {
            map.insert("reason_source".to_string(), json!(source));
        }
        map.insert("reason_details".to_string(), reason.details.clone());
    }
    map.insert("urgency".to_string(), json!(plan.urgency));
    map.insert("priority_route".to_string(), json!(plan.route));
    map.insert(
        "budget".to_string(),
        json!({
            "avoidable_loss_eth": budget.avoidable_loss_eth,
            "max_total_fee_eth": budget.max_total_fee_eth,
            "predicted_base_fee_gwei": budget.predicted_base_fee_gwei,
            "estimated_base_fee_cost_eth": budget.estimated_base_fee_cost_eth,
            "max_priority_spend_eth": budget.max_priority_spend_eth,
            "max_priority_fee_gwei": budget.max_priority_fee_gwei,
            "max_fee_per_gas_gwei": budget.max_fee_per_gas_gwei,
            "estimated_gas_used": budget.estimated_gas_used,
        }),
    );
    map.insert(
        "gas_plan".to_string(),
        json!({
            "label": gas_plan.label,
            "priority_fee_gwei": gas_plan.priority_fee_gwei,
            "max_fee_per_gas_gwei": gas_plan.max_fee_per_gas_gwei,
            "estimated_priority_spend_eth": gas_plan.estimated_priority_spend_eth,
            "estimated_max_cost_eth": gas_plan.estimated_max_cost_eth,
            "rank_position_p50": gas_plan.rank_position_p50,
            "gas_before_p50": gas_plan.gas_before_p50,
            "likely_fits_at_p50": gas_plan.likely_fits_at_p50,
            "source": gas_plan.source,
            "metadata": gas_plan.metadata,
        }),
    );
    map.insert(
        "gas_policy".to_string(),
        json!({
            "action": sell_gas_policy_action(plan),
            "signal": sell_gas_policy_signal(plan),
            "status": "selected",
            "profiles": gas_policy_profile_labels(gas_rank_policy),
            "submission_route": gas_rank_policy.submission_route.label(),
            "selected_profile": gas_plan.label,
            "gas_rank_source": gas_plan.source,
            "guard": "priority_fee_budget",
            "economic_guard": "protected_exit_value_minus_safety_buffer",
            "estimated_max_cost_eth": gas_plan.estimated_max_cost_eth,
            "estimated_priority_spend_eth": gas_plan.estimated_priority_spend_eth,
        }),
    );
    map.insert(
        "strategy_gas_rank_policy".to_string(),
        json!(gas_rank_policy),
    );
    map.insert(
        "route".to_string(),
        json!({
            "protocol": route.protocol,
            "router_address": route.router_address,
            "gas_limit": route.gas_limit,
            "estimated_gas_used": route.estimated_gas_used,
            "max_slippage_bps": route.max_slippage_bps,
        }),
    );
    map.insert("simulation".to_string(), simulation.metadata());
    if let Some(reason) = decision_reason {
        map.insert("decision_reason".to_string(), json!(reason));
    }
    if !source.is_null() {
        map.insert("source".to_string(), source);
    }

    Value::Object(map)
}

fn sell_gas_policy_action(plan: &PrioritySellPlan) -> &'static str {
    if plan.signal_source == LpSignalSource::MinedLiquidityRemoval {
        return "mined_liquidity_removal_exit";
    }
    match &plan.urgency {
        SellUrgency::NormalExit => "normal_exit",
        SellUrgency::MempoolPreMine => "mempool_race_exit",
        SellUrgency::MinedApprovalRace | SellUrgency::BuyConfirmBlockApproval => "lp_approval_exit",
    }
}

fn sell_gas_policy_signal(plan: &PrioritySellPlan) -> &'static str {
    match &plan.signal_source {
        LpSignalSource::MempoolLpApproval => "mempool_lp_approval",
        LpSignalSource::MinedLpApproval => "mined_lp_approval",
        LpSignalSource::MinedLiquidityRemoval => "mined_liquidity_removal",
        LpSignalSource::StrategyExit => "strategy_exit",
    }
}

fn gas_policy_profile_labels(policy: &StrategyGasRankPolicy) -> Vec<&'static str> {
    policy
        .preference_order
        .iter()
        .map(|profile| profile.label())
        .collect()
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use eth_alpha_core::{
        amount::DecimalAmount,
        decision_rationale::source,
        ids::{PoolAddress, TradeId},
    };

    use crate::{LpSignalSource, PriorityRoute, PrioritySellPlan, SellUrgency};

    use super::sell_gas_policy_action;

    #[test]
    fn mined_liquidity_removal_has_dedicated_gas_action_and_pool_update_source() {
        let plan = PrioritySellPlan {
            trade_id: TradeId("trade".to_string()),
            token_address: Address::with_last_byte(1),
            pool_address: PoolAddress::from("0xtoken:0xpool"),
            observed_block: 1,
            signal_source: LpSignalSource::MinedLiquidityRemoval,
            urgency: SellUrgency::MinedApprovalRace,
            route: PriorityRoute::Broadcast,
            max_priority_fee_per_gas_gwei: DecimalAmount::ZERO,
            max_total_fee_eth: DecimalAmount::ZERO,
            reason: "exit.liquidity_removal".to_string(),
        };

        assert_eq!(
            sell_gas_policy_action(&plan),
            "mined_liquidity_removal_exit"
        );
        assert_eq!(plan.event_source(), source::EVENT_SOURCE_POOL_UPDATE);
        assert_eq!(
            plan.decision_reason().unwrap().source.as_deref(),
            Some(source::EVENT_SOURCE_POOL_UPDATE)
        );
    }
}
