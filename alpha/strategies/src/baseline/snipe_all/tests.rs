use alloy_primitives::B256;
use eth_alpha_core::{
    execution::{ExecutionReport, ExecutionStatus},
    ids::OrderId,
    market::MarketSnapshotRef,
    order::OrderSide,
    portfolio::PortfolioState,
    risk::RISK_SOURCE_MEMPOOL_SIGNAL,
};
use rust_decimal::Decimal;

use super::*;

#[path = "test_support.rs"]
mod test_support;
use test_support::*;

#[test]
fn uses_configured_strategy_name() {
    let strategy = SnipeAllStrategy::new(SnipeAllConfig {
        strategy_name: StrategyName("snipe-all-hold20-pool-updates-liquidity-exit".to_string()),
        ..SnipeAllConfig::default()
    });

    assert_eq!(
        strategy.name(),
        StrategyName("snipe-all-hold20-pool-updates-liquidity-exit".to_string())
    );
}

#[test]
fn buys_every_eligible_pool_once() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let portfolio = PortfolioState::default();
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 1,
                pool: pool.clone(),
            },
        )
        .unwrap();
    match decision {
        StrategyDecision::SubmitOrder(intent)
        | StrategyDecision::SubmitOrderWithReason { intent, .. } => {
            assert_eq!(intent.side, OrderSide::Buy);
            assert_eq!(intent.pool_address, pool.address);
        }
        StrategyDecision::Hold
        | StrategyDecision::HoldWithReason { .. }
        | StrategyDecision::CancelOrders { .. } => {
            panic!("expected buy order")
        }
    }

    let repeat = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 2,
                pool,
            },
        )
        .unwrap();
    assert!(repeat.is_hold());
}

#[test]
fn lp_approval_entry_gate_blocks_above_threshold() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let portfolio = PortfolioState::default();
    let risks = vec![lp_approval_risk(
        &pool,
        "risk atlas mined-chain LP approval: count=1, generic_approved_pct=30.01%",
    )];
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        block_entry_on_lp_approval: true,
        lp_approval_gate_min_pct: Some(Decimal::from(30)),
        ..SnipeAllConfig::default()
    });

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 1,
                pool,
            },
        )
        .unwrap();

    assert!(decision.is_hold());
    assert_eq!(
        decision.reason(),
        Some("entry.lp_approval_gate:approved_pct_gt_min")
    );
}

#[test]
fn lp_approval_entry_gate_allows_at_threshold() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let portfolio = PortfolioState::default();
    let risks = vec![lp_approval_risk(
        &pool,
        "risk atlas mined-chain LP approval: count=1, generic_approved_pct=30.00%",
    )];
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        block_entry_on_lp_approval: true,
        lp_approval_gate_min_pct: Some(Decimal::from(30)),
        ..SnipeAllConfig::default()
    });

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 1,
                pool,
            },
        )
        .unwrap();

    assert!(decision.order_intent().is_some());
}

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

#[test]
fn buys_usd_stable_pool_at_stable_liquidity_floor() {
    let mut pool = pool();
    pool.denom_symbol = Some("USDC".to_string());
    pool.denom_reserve = Decimal::from(1_000u64);
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let portfolio = PortfolioState::default();
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 1,
                pool,
            },
        )
        .unwrap();

    assert!(decision.order_intent().is_some());
}

#[test]
fn holds_usd_stable_pool_below_stable_liquidity_floor() {
    let mut pool = pool();
    pool.denom_symbol = Some("USDT".to_string());
    pool.denom_reserve = Decimal::from(999u64);
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let portfolio = PortfolioState::default();
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 1,
                pool,
            },
        )
        .unwrap();

    assert!(decision.is_hold());
}

#[test]
fn buys_dai_pool_at_stable_liquidity_floor() {
    let mut pool = pool();
    pool.denom_symbol = Some("DAI".to_string());
    pool.denom_reserve = Decimal::from(1_000u64);
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let portfolio = PortfolioState::default();
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 1,
                pool,
            },
        )
        .unwrap();

    assert!(decision.order_intent().is_some());
}

#[test]
fn holds_weth_pool_below_eth_liquidity_floor() {
    let mut pool = pool();
    pool.denom_reserve = Decimal::new(49, 2);
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let portfolio = PortfolioState::default();
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 1,
                pool,
            },
        )
        .unwrap();

    assert!(decision.is_hold());
}

#[test]
fn restored_open_position_prevents_duplicate_buy() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());
    let position = confirmed_position(&strategy, &pool);
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 2,
                pool,
            },
        )
        .unwrap();
    assert!(decision.is_hold());
}

#[test]
fn restored_seen_pool_prevents_duplicate_buy_after_closed_position() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let portfolio = PortfolioState::default();
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy =
        SnipeAllStrategy::with_bought_pools(SnipeAllConfig::default(), vec![pool.address.clone()]);

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 2,
                pool,
            },
        )
        .unwrap();

    assert!(decision.is_hold());
    assert_eq!(
        decision.reason(),
        Some("entry.buy_eligible_pool_once:pool already bought")
    );
}

#[test]
fn sells_restored_open_position_on_liquidity_removal() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());
    let position = confirmed_position(&strategy, &pool);
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let risk = RiskEvent {
        kind: RiskKind::LiquidityRemoval,
        severity: RiskSeverity::Critical,
        source: None,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        pending_tx_hash: None,
        observed_block: Some(2),
        message: "liquidity removal".to_string(),
    };

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();
    assert_eq!(decision.reason(), Some("exit.liquidity_removal"));
    match decision {
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
fn mempool_liquidity_removal_signal_uses_explicit_exit_reason() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());
    let position = confirmed_position(&strategy, &pool);
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let risk = RiskEvent {
        kind: RiskKind::LiquidityRemoval,
        severity: RiskSeverity::Critical,
        source: Some(RISK_SOURCE_MEMPOOL_SIGNAL.to_string()),
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        pending_tx_hash: Some(B256::repeat_byte(0x33)),
        observed_block: Some(2),
        message: "liquidity removal signal".to_string(),
    };

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();

    assert_eq!(
        decision.reason(),
        Some("exit.mempool_liquidity_removal_signal")
    );
    assert_eq!(
        decision.order_intent().map(|intent| intent.side),
        Some(OrderSide::Sell)
    );
}

#[test]
fn skips_liquidity_removal_exit_when_pool_is_already_dust() {
    let mut pool = pool();
    pool.denom_reserve = Decimal::new(1, 3);
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());
    let position = confirmed_position(&strategy, &pool);
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let risk = RiskEvent {
        kind: RiskKind::LiquidityRemoval,
        severity: RiskSeverity::Critical,
        source: None,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        pending_tx_hash: None,
        observed_block: Some(2),
        message: "liquidity removal".to_string(),
    };

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();

    assert!(decision.is_hold());
    assert_eq!(
        decision.reason(),
        Some("exit.liquidity_removal:pool_denom_reserve_below_min_sell_threshold:0.001<0.01")
    );
}

#[test]
fn sells_open_position_on_lp_approval() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());
    let position = confirmed_position(&strategy, &pool);
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let risk = RiskEvent {
        kind: RiskKind::LpApproval,
        severity: RiskSeverity::Warning,
        source: None,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        pending_tx_hash: None,
        observed_block: Some(2),
        message: "lp approval".to_string(),
    };

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();
    match decision {
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
fn buy_confirm_block_lp_approval_can_defer_to_max_hold() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 2,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        defer_buy_confirm_block_lp_approval_to_max_hold: true,
        max_hold_blocks: Some(15),
        ..SnipeAllConfig::default()
    });
    let position = confirmed_position(&strategy, &pool);
    assert_eq!(position.entry_block, Some(1));
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut risk = lp_approval_risk(
        &pool,
        "risk atlas mined-chain LP approval: count=1, generic_approved_pct=100.00%",
    );
    risk.observed_block = Some(1);

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();

    assert!(decision.is_hold());
    assert_eq!(
        decision.reason(),
        Some("exit.lp_approval_buy_confirm_block_deferred_to_max_hold")
    );
}

#[test]
fn buy_confirm_block_defer_does_not_hide_later_lp_approval() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 3,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        defer_buy_confirm_block_lp_approval_to_max_hold: true,
        max_hold_blocks: Some(15),
        ..SnipeAllConfig::default()
    });
    let position = confirmed_position(&strategy, &pool);
    assert_eq!(position.entry_block, Some(1));
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut risk = lp_approval_risk(
        &pool,
        "risk atlas mined-chain LP approval: count=1, generic_approved_pct=100.00%",
    );
    risk.observed_block = Some(3);

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();

    match decision {
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
fn lp_approval_exit_gate_holds_below_threshold() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        lp_approval_gate_min_pct: Some(Decimal::from(30)),
        ..SnipeAllConfig::default()
    });
    let position = confirmed_position(&strategy, &pool);
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let risk = lp_approval_risk(
        &pool,
        "risk atlas mined-chain LP approval: count=1, generic_approved_pct=30.00%",
    );

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();

    assert!(decision.is_hold());
    assert_eq!(
        decision.reason(),
        Some("exit.lp_approval:approved_pct_unknown_or_not_gt_min")
    );
}

#[test]
fn lp_approval_exit_gate_sells_above_threshold() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        lp_approval_gate_min_pct: Some(Decimal::from(30)),
        ..SnipeAllConfig::default()
    });
    let position = confirmed_position(&strategy, &pool);
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let risk = lp_approval_risk(
        &pool,
        "risk atlas mined-chain LP approval: count=1, generic_approved_pct=30.01%",
    );

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();

    match decision {
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
