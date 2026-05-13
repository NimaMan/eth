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

#[test]
fn failed_buy_has_no_exposure() {
    let mut position = position();
    let order_id = OrderId("buy-1".to_string());

    position.mark_intent_created(OrderSide::Buy).unwrap();
    position
        .mark_order_submitted(order_id.clone(), OrderSide::Buy)
        .unwrap();
    position
        .apply_execution_report(&ExecutionReport {
            order_id,
            status: ExecutionStatus::Failed,
            tx_hash: None,
            block_number: Some(1),
            filled_amount: None,
            token_amount: None,
            gas_used: None,
            error: Some("reverted".to_string()),
        })
        .unwrap();

    assert_eq!(position.state, PositionState::BuyFailed);
    assert!(!position.has_exposure());
    assert!(position.state.is_terminal());
}

#[test]
fn failed_sell_keeps_exposure_and_can_retry() {
    let mut position = position();
    let buy_order_id = OrderId("buy-1".to_string());
    let sell_order_id = OrderId("sell-1".to_string());

    position.mark_intent_created(OrderSide::Buy).unwrap();
    position
        .mark_order_submitted(buy_order_id.clone(), OrderSide::Buy)
        .unwrap();
    position
        .apply_execution_report(&ExecutionReport {
            order_id: buy_order_id,
            status: ExecutionStatus::Confirmed,
            tx_hash: Some(B256::ZERO),
            block_number: Some(1),
            filled_amount: Some(Amount::zero(18)),
            token_amount: Some(Amount::zero(18)),
            gas_used: Some(21_000),
            error: None,
        })
        .unwrap();

    position.mark_intent_created(OrderSide::Sell).unwrap();
    position
        .mark_order_submitted(sell_order_id.clone(), OrderSide::Sell)
        .unwrap();
    position
        .apply_execution_report(&ExecutionReport {
            order_id: sell_order_id,
            status: ExecutionStatus::Failed,
            tx_hash: None,
            block_number: Some(2),
            filled_amount: None,
            token_amount: None,
            gas_used: None,
            error: Some("locked".to_string()),
        })
        .unwrap();

    assert_eq!(position.state, PositionState::SellFailed);
    assert!(position.has_exposure());
    assert!(position.can_submit_exit());
    assert!(!position.state.is_terminal());

    assert!(position.mark_intent_created(OrderSide::Sell).is_ok());
    assert_eq!(position.state, PositionState::SellIntentCreated);
}

#[test]
fn simulator_infra_sell_failure_keeps_exposure_but_blocks_retry() {
    let mut position = position();
    let buy_order_id = OrderId("buy-1".to_string());
    let sell_order_id = OrderId("sell-1".to_string());

    position.mark_intent_created(OrderSide::Buy).unwrap();
    position
        .mark_order_submitted(buy_order_id.clone(), OrderSide::Buy)
        .unwrap();
    position
        .apply_execution_report(&ExecutionReport {
            order_id: buy_order_id,
            status: ExecutionStatus::Confirmed,
            tx_hash: Some(B256::ZERO),
            block_number: Some(1),
            filled_amount: Some(Amount::zero(18)),
            token_amount: Some(Amount::zero(18)),
            gas_used: Some(21_000),
            error: None,
        })
        .unwrap();

    position.mark_intent_created(OrderSide::Sell).unwrap();
    position
        .mark_order_submitted(sell_order_id.clone(), OrderSide::Sell)
        .unwrap();
    position
        .apply_execution_report(&ExecutionReport {
            order_id: sell_order_id,
            status: ExecutionStatus::Failed,
            tx_hash: None,
            block_number: Some(2),
            filled_amount: None,
            token_amount: None,
            gas_used: None,
            error: Some(
                "unable to inject synthetic ERC20 balance: unsupported balance storage layout"
                    .to_string(),
            ),
        })
        .unwrap();

    assert_eq!(position.state, PositionState::SellFailed);
    assert!(position.has_exposure());
    assert!(!position.can_submit_exit());
    assert!(!position.state.is_terminal());
    assert_eq!(
        position.exit_failure_reason.as_deref(),
        Some("unable to inject synthetic ERC20 balance: unsupported balance storage layout")
    );
    assert!(position.mark_intent_created(OrderSide::Sell).is_err());
    assert_eq!(position.state, PositionState::SellFailed);
}
