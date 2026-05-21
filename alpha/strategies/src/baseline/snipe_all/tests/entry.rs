use super::*;
use alloy_primitives::{Address, U256};
use eth_alpha_core::{
    amount::Amount,
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, TokenPoolId},
};

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
fn price_to_initial_entry_gate_blocks_above_threshold() {
    let mut pool = pool();
    pool.price_ratio_to_initial = Some(Decimal::new(151, 2));
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
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        max_entry_price_ratio_to_initial: Some(Decimal::new(15, 1)),
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
        Some("entry.price_to_initial_ratio_gt_max")
    );
}

#[test]
fn price_to_initial_entry_gate_allows_at_threshold_and_missing_ratio() {
    for price_ratio_to_initial in [Some(Decimal::new(15, 1)), None] {
        let mut pool = pool();
        pool.price_ratio_to_initial = price_ratio_to_initial;
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
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
            max_entry_price_ratio_to_initial: Some(Decimal::new(15, 1)),
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
fn entry_bankroll_blocks_buy_when_restored_seen_pool_consumes_bankroll() {
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
    let mut strategy = SnipeAllStrategy::with_bought_pools(
        SnipeAllConfig {
            entry_bankroll_wei: Some(U256::from(10_000_000_000_000_000u64)),
            ..SnipeAllConfig::default()
        },
        vec![TokenPoolId::new(
            Address::repeat_byte(0xaa),
            Address::repeat_byte(0xbb).to_string(),
        )],
    );

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
    assert_eq!(decision.reason(), Some("entry.bankroll_insufficient"));
}

#[test]
fn entry_bankroll_allows_redeploying_confirmed_profit() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 3,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let risks = Vec::new();
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        entry_bankroll_wei: Some(U256::from(10_000_000_000_000_000u64)),
        ..SnipeAllConfig::default()
    });

    let mut closed_pool = pool.clone();
    closed_pool.token_address = Address::repeat_byte(0xcc);
    closed_pool.address = TokenPoolId::new(
        closed_pool.token_address,
        Address::repeat_byte(0xdd).to_string(),
    );
    let mut closed = confirmed_position(&strategy, &closed_pool);
    closed.entry_cost_basis = Some(Decimal::new(1, 2));
    closed.mark_intent_created(OrderSide::Sell).unwrap();
    closed
        .mark_order_submitted(OrderId("sell-profit".to_string()), OrderSide::Sell)
        .unwrap();
    closed
        .apply_execution_report(&ExecutionReport {
            order_id: OrderId("sell-profit".to_string()),
            status: ExecutionStatus::Confirmed,
            tx_hash: None,
            block_number: Some(2),
            filled_amount: Some(Amount {
                raw: U256::from(20_000_000_000_000_000u64),
                decimals: 18,
            }),
            token_amount: None,
            gas_used: Some(120_000),
            gas_cost: None,
            mined_evidence: None,
            error: None,
        })
        .unwrap();

    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(closed.id.clone(), closed);
    let ctx = ctx(&market, &portfolio, &risks);

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 3,
                pool,
            },
        )
        .unwrap();

    assert!(decision.order_intent().is_some());
}
