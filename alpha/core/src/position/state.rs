use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PositionState {
    Init,
    BuyIntentCreated,
    BuySubmitted,
    BuyConfirmed,
    BuyDeferred,
    BuyFailed,
    BuyCancelled,
    SellIntentCreated,
    SellSubmitted,
    SellFailed,
    SellCancelled,
    SellConfirmed,
    Cancelled,
    /// Terminal, cause-agnostic "value is permanently zero" outcome: the
    /// position holds no recoverable proceeds (e.g. a confiscated/drained
    /// balance that can never be sold). This describes the OUTCOME only; the
    /// cause (a confirmed pool drain / scam) lives in pool-state flags and risk
    /// events, not in the position lifecycle. Accepts the legacy `"Scammed"`
    /// serialized form for back-compat with already-persisted rows.
    #[serde(alias = "Scammed")]
    TerminalZero,
}

impl PositionState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PositionState::BuyFailed
                | PositionState::BuyDeferred
                | PositionState::BuyCancelled
                | PositionState::SellConfirmed
                | PositionState::Cancelled
                | PositionState::TerminalZero
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
