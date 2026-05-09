use crate::{
    order::OrderIntent,
    risk::{RiskDecision, RiskEvent},
};

pub trait RiskPolicy: Send + Sync {
    fn evaluate_order(&self, intent: &OrderIntent, active_risks: &[RiskEvent]) -> RiskDecision;
}
