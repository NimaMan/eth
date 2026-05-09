use crate::order::OrderIntent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StrategyDecision {
    Hold,
    SubmitOrder(OrderIntent),
    CancelOrders { reason: String },
}
