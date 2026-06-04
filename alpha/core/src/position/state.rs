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
    /// Terminal, cause-agnostic "closed at zero value" outcome: the position
    /// holds no recoverable proceeds. This describes the OUTCOME only; the cause
    /// (for example, a confirmed pool drain / scam) lives in pool-state flags and
    /// risk events, not in the position lifecycle. Accepts legacy serialized
    /// forms for back-compat with already-persisted rows.
    #[serde(
        rename = "closed_zero_valuation",
        alias = "ClosedZeroValuation",
        alias = "TerminalZero",
        alias = "terminal_zero",
        alias = "Scammed",
        alias = "scammed"
    )]
    ClosedZeroValuation,
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
                | PositionState::ClosedZeroValuation
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
