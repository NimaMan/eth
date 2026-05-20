use eth_alpha_core::{
    order::OrderSide, risk::RiskKind, store::StrategyDecisionRecord, strategy::StrategyDecision,
};
use serde_json::json;

pub(crate) fn strategy_decision_record(
    strategy_name: &str,
    event_source: &str,
    event_key: String,
    block_number: Option<u64>,
    token_address: Option<String>,
    pool_address: Option<String>,
    decision: &StrategyDecision,
) -> StrategyDecisionRecord {
    let order = decision.order_intent();
    let action = strategy_decision_action(decision);
    let structured_reason = decision.structured_reason(Some(event_source), Some(action));
    StrategyDecisionRecord {
        strategy_name: strategy_name.to_string(),
        event_source: event_source.to_string(),
        event_key,
        block_number,
        token_address: token_address
            .or_else(|| order.map(|intent| intent.token_address.to_string())),
        pool_address: pool_address.or_else(|| order.map(|intent| intent.pool_address.to_string())),
        action: action.to_string(),
        reason: decision.reason().map(ToOwned::to_owned),
        reason_code: structured_reason.as_ref().map(|reason| reason.code.clone()),
        reason_category: structured_reason
            .as_ref()
            .map(|reason| reason.category_key().to_string()),
        reason_label: structured_reason
            .as_ref()
            .map(|reason| reason.label.clone()),
        reason_source: structured_reason
            .as_ref()
            .and_then(|reason| reason.source.clone()),
        reason_details: structured_reason
            .as_ref()
            .map(|reason| reason.details.clone()),
        order_side: order.map(|intent| intent.side),
        payload: json!({
            "decision": decision,
            "reason": structured_reason,
        }),
    }
}

pub(crate) fn strategy_decision_action(decision: &StrategyDecision) -> &'static str {
    match decision {
        StrategyDecision::Hold | StrategyDecision::HoldWithReason { .. } => "hold",
        StrategyDecision::SubmitOrder(intent)
        | StrategyDecision::SubmitOrderWithReason { intent, .. } => match intent.side {
            OrderSide::Buy => "submit_buy",
            OrderSide::Sell => "submit_sell",
        },
        StrategyDecision::CancelOrders { .. } => "cancel_orders",
    }
}

pub(crate) fn risk_kind_key(kind: &RiskKind) -> String {
    match kind {
        RiskKind::LiquidityRemoval => "liquidity_removal".to_string(),
        RiskKind::MempoolLiquidityRemoval => "mempool_liquidity_removal".to_string(),
        RiskKind::TaxChange => "tax_change".to_string(),
        RiskKind::Honeypot => "honeypot".to_string(),
        RiskKind::TradingDisabled => "trading_disabled".to_string(),
        RiskKind::TradingEnabled => "trading_enabled".to_string(),
        RiskKind::LpApproval => "lp_approval".to_string(),
        RiskKind::ScamConfirmed => "scam_confirmed".to_string(),
        RiskKind::Custom(value) => value.clone(),
    }
}
