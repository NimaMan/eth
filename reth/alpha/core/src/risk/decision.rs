use crate::order::OrderIntent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RiskDecision {
    Allow,
    Reject {
        reason: String,
    },
    ReduceSize {
        reason: String,
    },
    CancelOpenOrders {
        reason: String,
    },
    ForceExit {
        reason: String,
        intent: Box<OrderIntent>,
    },
}
