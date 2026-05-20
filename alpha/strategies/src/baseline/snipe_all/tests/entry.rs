use super::*;

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
