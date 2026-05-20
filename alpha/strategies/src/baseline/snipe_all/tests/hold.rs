use super::*;

#[test]
fn position_monitor_does_not_exit_at_max_hold_without_pool_update() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 201,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut portfolio = PortfolioState::default();
    let risks = Vec::new();
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        max_hold_blocks: Some(200),
        ..SnipeAllConfig::default()
    });
    let position = confirmed_position(&strategy, &pool);
    portfolio.positions.insert(position.id.clone(), position);
    let ctx = ctx(&market, &portfolio, &risks);

    let decisions = strategy.on_position_monitor(&ctx, 201).unwrap();

    assert!(decisions.is_empty());
}

#[test]
fn max_hold_counts_active_pool_update_blocks_not_chain_blocks() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 100,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut portfolio = PortfolioState::default();
    let risks = Vec::new();
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        max_hold_blocks: Some(2),
        ..SnipeAllConfig::default()
    });
    let position = confirmed_position(&strategy, &pool);
    portfolio.positions.insert(position.id.clone(), position);
    let ctx = ctx(&market, &portfolio, &risks);

    let first_active_block = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 100,
                pool: pool.clone(),
            },
        )
        .unwrap();

    assert!(first_active_block.is_hold());

    let second_active_block = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 101,
                pool: pool.clone(),
            },
        )
        .unwrap();

    match second_active_block {
        StrategyDecision::SubmitOrder(intent)
        | StrategyDecision::SubmitOrderWithReason { intent, .. } => {
            assert_eq!(intent.side, OrderSide::Sell);
            assert_eq!(intent.pool_address, pool.address);
        }
        StrategyDecision::Hold
        | StrategyDecision::HoldWithReason { .. }
        | StrategyDecision::CancelOrders { .. } => {
            panic!("expected sell order")
        }
    }
}

#[test]
fn max_hold_counts_each_active_block_once() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 100,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut portfolio = PortfolioState::default();
    let risks = Vec::new();
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        max_hold_blocks: Some(2),
        ..SnipeAllConfig::default()
    });
    let position = confirmed_position(&strategy, &pool);
    portfolio.positions.insert(position.id.clone(), position);
    let ctx = ctx(&market, &portfolio, &risks);

    let first = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 100,
                pool: pool.clone(),
            },
        )
        .unwrap();
    let duplicate = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 100,
                pool: pool.clone(),
            },
        )
        .unwrap();

    assert!(first.is_hold());
    assert!(duplicate.is_hold());

    let next_block = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 101,
                pool: pool.clone(),
            },
        )
        .unwrap();

    assert!(!next_block.is_hold());
}

#[test]
fn restored_active_hold_counter_triggers_max_hold_exit() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 101,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut portfolio = PortfolioState::default();
    let risks = Vec::new();
    let strategy_for_position = SnipeAllStrategy::new(SnipeAllConfig::default());
    let position = confirmed_position(&strategy_for_position, &pool);
    let position_id = position.id.clone();
    portfolio.positions.insert(position.id.clone(), position);
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::with_restored_state(
        SnipeAllConfig {
            max_hold_blocks: Some(2),
            ..SnipeAllConfig::default()
        },
        vec![pool.address.clone()],
        vec![(position_id, 1, Some(100))],
    );

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 101,
                pool: pool.clone(),
            },
        )
        .unwrap();

    match decision {
        StrategyDecision::SubmitOrder(intent)
        | StrategyDecision::SubmitOrderWithReason { intent, .. } => {
            assert_eq!(intent.side, OrderSide::Sell);
            assert_eq!(intent.pool_address, pool.address);
        }
        StrategyDecision::Hold
        | StrategyDecision::HoldWithReason { .. }
        | StrategyDecision::CancelOrders { .. } => {
            panic!("expected restored counter to trigger sell")
        }
    }
}

#[test]
fn position_monitor_exits_restored_active_hold_over_limit_without_pool_update() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 250,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: None,
    };
    let mut portfolio = PortfolioState::default();
    let risks = Vec::new();
    let strategy_for_position = SnipeAllStrategy::new(SnipeAllConfig::default());
    let position = confirmed_position(&strategy_for_position, &pool);
    let position_id = position.id.clone();
    portfolio.positions.insert(position.id.clone(), position);
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::with_restored_state(
        SnipeAllConfig {
            max_hold_blocks: Some(12),
            ..SnipeAllConfig::default()
        },
        vec![pool.address.clone()],
        vec![(position_id, 12, Some(200))],
    );

    let decisions = strategy.on_position_monitor(&ctx, 250).unwrap();

    assert_eq!(decisions.len(), 1);
    assert_eq!(
        decisions[0].reason(),
        Some("exit.max_hold_active_blocks_restored")
    );
    assert_eq!(decisions[0].order_intent().unwrap().side, OrderSide::Sell);
}

#[test]
fn position_monitor_does_not_retry_failed_exit_every_block() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 250,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut portfolio = PortfolioState::default();
    let risks = Vec::new();
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        max_hold_blocks: Some(200),
        ..SnipeAllConfig::default()
    });
    let mut position = confirmed_position(&strategy, &pool);
    position.mark_intent_created(OrderSide::Sell).unwrap();
    position
        .mark_order_submitted(OrderId("sell-1".to_string()), OrderSide::Sell)
        .unwrap();
    position
        .apply_execution_report(&ExecutionReport {
            order_id: OrderId("sell-1".to_string()),
            status: ExecutionStatus::Failed,
            tx_hash: None,
            block_number: Some(202),
            filled_amount: None,
            token_amount: None,
            gas_used: Some(21_000),
            gas_cost: None,
            error: Some("temporary sell failure".to_string()),
        })
        .unwrap();
    assert!(position.can_submit_exit());
    portfolio.positions.insert(position.id.clone(), position);
    let ctx = ctx(&market, &portfolio, &risks);

    let decisions = strategy.on_position_monitor(&ctx, 250).unwrap();

    assert!(decisions.is_empty());
}

#[test]
fn market_event_does_not_retry_failed_exit() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 250,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut portfolio = PortfolioState::default();
    let risks = Vec::new();
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        max_hold_blocks: Some(200),
        ..SnipeAllConfig::default()
    });
    let position = failed_exit_position(&strategy, &pool);
    portfolio.positions.insert(position.id.clone(), position);
    let ctx = ctx(&market, &portfolio, &risks);

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 250,
                pool,
            },
        )
        .unwrap();

    assert!(decision.is_hold());
    assert_eq!(
        decision.reason(),
        Some("position_exit_failed_no_strategy_retry")
    );
}
