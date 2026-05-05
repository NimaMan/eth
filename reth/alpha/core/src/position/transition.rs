use crate::position::PositionState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PositionTransition {
    pub from: PositionState,
    pub to: PositionState,
    pub reason: String,
}
