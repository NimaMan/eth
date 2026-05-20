use crate::{decision_rationale::DecisionReason, order::OrderIntent};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StrategyDecision {
    Hold,
    HoldWithReason { reason: String },
    SubmitOrder(OrderIntent),
    SubmitOrderWithReason { intent: OrderIntent, reason: String },
    CancelOrders { reason: String },
}

impl StrategyDecision {
    pub fn hold(reason: impl Into<String>) -> Self {
        Self::HoldWithReason {
            reason: reason.into(),
        }
    }

    pub fn submit_order(intent: OrderIntent, reason: impl Into<String>) -> Self {
        Self::SubmitOrderWithReason {
            intent,
            reason: reason.into(),
        }
    }

    pub fn is_hold(&self) -> bool {
        matches!(self, Self::Hold | Self::HoldWithReason { .. })
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Hold => None,
            Self::HoldWithReason { reason }
            | Self::SubmitOrderWithReason { reason, .. }
            | Self::CancelOrders { reason } => Some(reason.as_str()),
            Self::SubmitOrder(_) => None,
        }
    }

    pub fn structured_reason(
        &self,
        event_source: Option<&str>,
        action: Option<&str>,
    ) -> Option<DecisionReason> {
        DecisionReason::from_parts(self.reason(), event_source, action)
    }

    pub fn order_intent(&self) -> Option<&OrderIntent> {
        match self {
            Self::SubmitOrder(intent) | Self::SubmitOrderWithReason { intent, .. } => Some(intent),
            Self::Hold | Self::HoldWithReason { .. } | Self::CancelOrders { .. } => None,
        }
    }
}
