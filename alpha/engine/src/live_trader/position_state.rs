use eth_alpha_core::position::{Position, PositionState};

pub(super) fn release_stale_submitted_position(position: &mut Position) -> bool {
    match position.state {
        PositionState::SellSubmitted | PositionState::SellIntentCreated => {
            position.state = PositionState::BuyConfirmed;
            position.exit_order_id = None;
            position.exit_failure_reason = None;
            position.exit_retryable = true;
            true
        }
        PositionState::BuySubmitted | PositionState::BuyIntentCreated => {
            position.state = PositionState::BuyFailed;
            true
        }
        _ => false,
    }
}
