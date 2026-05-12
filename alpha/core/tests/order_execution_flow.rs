use eth_alpha_core::{ExecutionReport, ExecutionStatus, OrderId, PositionState};

#[test]
fn failed_buy_status_is_terminal_but_failed_sell_keeps_exposure() {
    assert!(PositionState::BuyFailed.is_terminal());
    assert!(!PositionState::SellFailed.is_terminal());
    assert!(PositionState::SellFailed.has_exposure());
    assert!(PositionState::SellFailed.can_submit_exit());
    assert_eq!(ExecutionStatus::Failed, ExecutionStatus::Failed);

    let report = ExecutionReport {
        order_id: OrderId("order-1".to_string()),
        status: ExecutionStatus::Failed,
        tx_hash: None,
        block_number: None,
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        error: Some("reverted".to_string()),
    };

    assert_eq!(report.status, ExecutionStatus::Failed);
}
