use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrderStatus {
    IntentCreated,
    Submitted,
    Confirmed,
    Failed,
    Cancelled,
}
