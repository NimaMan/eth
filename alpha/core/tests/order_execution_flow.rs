use eth_alpha_core::{ExecutionReport, ExecutionStatus, OrderId, PositionState};

#[test]
fn failed_execution_status_is_terminal() {
    assert!(PositionState::Failed.is_terminal());
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
