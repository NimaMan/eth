use eth_alpha_core::{
    order::{OrderIntent, OrderSide},
    risk::{RiskDecision, RiskEvent, RiskKind, RiskPolicy, RiskSeverity},
};

#[derive(Clone, Debug, Default)]
pub struct AllowAllRiskPolicy;

impl RiskPolicy for AllowAllRiskPolicy {
    fn evaluate_order(&self, _intent: &OrderIntent, _active_risks: &[RiskEvent]) -> RiskDecision {
        RiskDecision::Allow
    }
}

#[derive(Clone, Debug, Default)]
pub struct BlockCriticalRiskPolicy;

impl RiskPolicy for BlockCriticalRiskPolicy {
    fn evaluate_order(&self, intent: &OrderIntent, active_risks: &[RiskEvent]) -> RiskDecision {
        if intent.side == OrderSide::Sell {
            return RiskDecision::Allow;
        }
        if let Some(risk) = active_risks.iter().rev().find(|risk| {
            risk.severity == RiskSeverity::Critical
                && risk.kind != RiskKind::TradingEnabled
                && risk.token_address == intent.token_address
                && risk
                    .pool_address
                    .as_ref()
                    .map(|pool| pool == &intent.pool_address)
                    .unwrap_or(true)
        }) {
            return RiskDecision::Reject {
                reason: format!("critical active risk for order: {}", risk.message),
            };
        }
        RiskDecision::Allow
    }
}
