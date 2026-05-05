use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PositionState {
    Init,
    BuyIntentCreated,
    BuySubmitted,
    BuyConfirmed,
    SellIntentCreated,
    SellSubmitted,
    SellConfirmed,
    Failed,
    Cancelled,
    Scammed,
}

impl PositionState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PositionState::SellConfirmed
                | PositionState::Failed
                | PositionState::Cancelled
                | PositionState::Scammed
        )
    }
}
