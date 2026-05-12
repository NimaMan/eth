use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PositionState {
    Init,
    BuyIntentCreated,
    BuySubmitted,
    BuyConfirmed,
    BuyFailed,
    BuyCancelled,
    SellIntentCreated,
    SellSubmitted,
    SellFailed,
    SellCancelled,
    SellConfirmed,
    Cancelled,
    Scammed,
}

impl PositionState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PositionState::BuyFailed
                | PositionState::BuyCancelled
                | PositionState::SellConfirmed
                | PositionState::Cancelled
                | PositionState::Scammed
        )
    }

    pub fn has_exposure(&self) -> bool {
        matches!(
            self,
            PositionState::BuyConfirmed
                | PositionState::SellIntentCreated
                | PositionState::SellSubmitted
                | PositionState::SellFailed
                | PositionState::SellCancelled
        )
    }

    pub fn can_submit_exit(&self) -> bool {
        matches!(
            self,
            PositionState::BuyConfirmed | PositionState::SellFailed | PositionState::SellCancelled
        )
    }
}
