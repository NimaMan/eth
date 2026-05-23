use super::*;

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
        kind: RiskKind::MempoolLiquidityRemoval,
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
fn early_launch_lp_approval_can_defer_to_max_hold() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 3,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        lp_approval_gate_min_pct: Some(Decimal::from(30)),
        lp_approval_exit_defer_max_trading_enabled_age_blocks: Some(2),
        max_hold_blocks: Some(15),
        ..SnipeAllConfig::default()
    });
    let position = confirmed_position(&strategy, &pool);
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let risk = lp_approval_risk(
        &pool,
        "risk atlas mined-chain LP approval: count=1, generic_approved_pct=100.00%, trading_enabled_to_last_lp_approval_chain_block_delta=2",
    );

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();

    assert!(decision.is_hold());
    assert_eq!(
        decision.reason(),
        Some("exit.lp_approval:early_approval_deferred_to_max_hold")
    );
}

#[test]
fn late_lp_approval_exits_under_launch_window_policy() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 4,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        lp_approval_gate_min_pct: Some(Decimal::from(30)),
        lp_approval_exit_defer_max_trading_enabled_age_blocks: Some(2),
        max_hold_blocks: Some(15),
        ..SnipeAllConfig::default()
    });
    let position = confirmed_position(&strategy, &pool);
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let risk = lp_approval_risk(
        &pool,
        "risk atlas mined-chain LP approval: count=1, generic_approved_pct=100.00%, trading_enabled_to_last_lp_approval_chain_block_delta=3",
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
