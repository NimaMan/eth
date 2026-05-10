use alloy_primitives::{Address, B256};
use eth_alpha_core::{
    Amount, ExecutionReport, ExecutionStatus, OrderId, OrderSide, PortfolioId, Position,
    PositionId, PositionKey, PositionState, StrategyName, TokenPoolId, WalletId,
};

fn position() -> Position {
    Position::new(
        PositionId("pos-1".to_string()),
        PositionKey {
            portfolio_id: PortfolioId("portfolio".to_string()),
            wallet_id: WalletId("wallet".to_string()),
            strategy_name: StrategyName("strategy".to_string()),
            token_address: Address::ZERO,
            pool_address: TokenPoolId::new(Address::ZERO, Address::with_last_byte(1).to_string()),
        },
    )
}

#[test]
fn buy_must_submit_before_confirm() {
    let mut position = position();
    let report = ExecutionReport {
        order_id: OrderId("buy-1".to_string()),
        status: ExecutionStatus::Confirmed,
        tx_hash: Some(B256::ZERO),
        block_number: Some(1),
        filled_amount: Some(Amount::zero(18)),
        token_amount: None,
        gas_used: Some(21_000),
        error: None,
    };

    assert!(position.apply_execution_report(&report).is_err());
    assert_eq!(position.state, PositionState::Init);
}

#[test]
fn buy_submit_confirm_flow_reaches_buy_confirmed() {
    let mut position = position();
    let order_id = OrderId("buy-1".to_string());

    position.mark_intent_created(OrderSide::Buy).unwrap();
    position
        .mark_order_submitted(order_id.clone(), OrderSide::Buy)
        .unwrap();
    position
        .apply_execution_report(&ExecutionReport {
            order_id,
            status: ExecutionStatus::Confirmed,
            tx_hash: Some(B256::ZERO),
            block_number: Some(1),
            filled_amount: Some(Amount::zero(18)),
            token_amount: None,
            gas_used: Some(21_000),
            error: None,
        })
        .unwrap();

    assert_eq!(position.state, PositionState::BuyConfirmed);
}

#[test]
fn sell_requires_confirmed_buy() {
    let mut position = position();

    assert!(position.mark_intent_created(OrderSide::Sell).is_err());
    assert_eq!(position.state, PositionState::Init);
}
