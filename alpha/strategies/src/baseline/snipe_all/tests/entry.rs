use super::*;
use crate::shared_rules::entry::init_policy::EntryInitPolicyConfig;
use alloy_primitives::{Address, U256};
use eth_alpha_core::{
    amount::Amount,
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, PositionId, TokenPoolId},
    mempool_entry::{MEMPOOL_ENTRY_EVIDENCE_KEY, MEMPOOL_ENTRY_EVIDENCE_VERSION},
    position::{Position, PositionKey},
    risk::RISK_SOURCE_MEMPOOL_SIGNAL,
};
use serde_json::{json, Value};

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
        entry_init_policy: EntryInitPolicyConfig {
            max_price_ratio_to_initial: Some(Decimal::new(15, 1)),
            ..EntryInitPolicyConfig::default()
        },
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
        Some("entry.init_policy:price_to_initial_ratio_gt_max")
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
            entry_init_policy: EntryInitPolicyConfig {
                max_price_ratio_to_initial: Some(Decimal::new(15, 1)),
                ..EntryInitPolicyConfig::default()
            },
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
fn trading_enabled_mempool_entry_evidence_buys_projected_pool() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 2,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: None,
    };
    let portfolio = PortfolioState::default();
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());
    let risk = trading_enabled_risk(
        &pool,
        Some(mempool_entry_evidence(&pool, Some(Decimal::new(12, 1)))),
    );

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();

    let intent = decision.order_intent().expect("buy intent");
    assert_eq!(intent.side, OrderSide::Buy);
    assert_eq!(intent.pool_address, pool.address);
    assert_eq!(decision.reason(), Some("entry.tail_after_enabling_tx"));
}

#[test]
fn trading_enabled_without_mempool_entry_evidence_holds() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 2,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: None,
    };
    let portfolio = PortfolioState::default();
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());
    let risk = trading_enabled_risk(&pool, None);

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();

    assert!(decision.is_hold());
    assert_eq!(
        decision.reason(),
        Some("entry.mempool_entry_evidence:missing_evidence")
    );
}

#[test]
fn trading_enabled_mempool_entry_evidence_reuses_price_ratio_gate() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 2,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: None,
    };
    let portfolio = PortfolioState::default();
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        entry_init_policy: EntryInitPolicyConfig {
            max_price_ratio_to_initial: Some(Decimal::new(15, 1)),
            ..EntryInitPolicyConfig::default()
        },
        ..SnipeAllConfig::default()
    });
    let risk = trading_enabled_risk(
        &pool,
        Some(mempool_entry_evidence(&pool, Some(Decimal::new(151, 2)))),
    );

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();

    assert!(decision.is_hold());
    assert_eq!(
        decision.reason(),
        Some("entry.init_policy:price_to_initial_ratio_gt_max")
    );
}

#[test]
fn trading_enabled_mempool_probe_without_exact_vault_buy_holds() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 2,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: None,
    };
    let portfolio = PortfolioState::default();
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());
    let mut evidence = mempool_entry_evidence(&pool, Some(Decimal::new(12, 1)));
    evidence["vault_buy_simulation"]["route"] = json!("pool_buy_sell_probe");
    let risk = trading_enabled_risk(&pool, Some(evidence));

    let decision = strategy.on_risk_event(&ctx, &risk).unwrap();

    assert!(decision.is_hold());
    assert_eq!(
        decision.reason(),
        Some("entry.mempool_entry_evidence:missing_successful_exact_vault_buy")
    );
}

#[test]
fn entry_init_policy_blocks_pool_age_above_threshold() {
    let mut pool = pool();
    pool.creation_block = Some(1);
    pool.latest_block = 30;
    let market = MarketSnapshotRef {
        block_number: 30,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let portfolio = PortfolioState::default();
    let risks = Vec::new();
    let ctx = ctx(&market, &portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
        entry_init_policy: EntryInitPolicyConfig {
            max_age_blocks: Some(20),
            ..EntryInitPolicyConfig::default()
        },
        ..SnipeAllConfig::default()
    });

    let decision = strategy
        .on_market_event(
            &ctx,
            &MarketEvent::PoolUpdated {
                block_number: 30,
                pool,
            },
        )
        .unwrap();

    assert!(decision.is_hold());
    assert_eq!(decision.reason(), Some("entry.init_policy:pool_age_gt_max"));
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
fn deferred_buy_attempt_can_retry_same_pool() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let empty_portfolio = PortfolioState::default();
    let risks = Vec::new();
    let empty_ctx = ctx(&market, &empty_portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

    assert!(strategy
        .on_market_event(
            &empty_ctx,
            &MarketEvent::PoolUpdated {
                block_number: 1,
                pool: pool.clone(),
            },
        )
        .unwrap()
        .order_intent()
        .is_some());

    let mut deferred = Position::new(
        PositionId("deferred-buy".to_string()),
        PositionKey {
            portfolio_id: strategy.config.portfolio_id.clone(),
            wallet_id: strategy.config.wallet_id.clone(),
            strategy_name: strategy.name(),
            token_address: pool.token_address,
            pool_address: pool.address.clone(),
            protocol: pool.protocol.clone(),
        },
    );
    deferred.mark_intent_created(OrderSide::Buy).unwrap();
    deferred
        .mark_order_submitted(OrderId("buy-1".to_string()), OrderSide::Buy)
        .unwrap();
    deferred
        .apply_execution_report(&ExecutionReport {
            order_id: OrderId("buy-1".to_string()),
            status: ExecutionStatus::Deferred,
            tx_hash: None,
            block_number: Some(1),
            filled_amount: None,
            token_amount: None,
            gas_used: None,
            gas_cost: None,
            mined_evidence: None,
            error: Some("simulation state not ready".to_string()),
        })
        .unwrap();

    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(deferred.id.clone(), deferred);
    let retry_ctx = ctx(&market, &portfolio, &risks);
    let retry = strategy
        .on_market_event(
            &retry_ctx,
            &MarketEvent::PoolUpdated {
                block_number: 2,
                pool,
            },
        )
        .unwrap();

    assert!(retry.order_intent().is_some());
    assert_eq!(
        retry.reason(),
        Some("entry.buy_eligible_pool_once:retry_after_noncapital_execution")
    );
}

#[test]
fn cancelled_buy_attempt_can_retry_same_pool() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 1,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let empty_portfolio = PortfolioState::default();
    let risks = Vec::new();
    let empty_ctx = ctx(&market, &empty_portfolio, &risks);
    let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

    assert!(strategy
        .on_market_event(
            &empty_ctx,
            &MarketEvent::PoolUpdated {
                block_number: 1,
                pool: pool.clone(),
            },
        )
        .unwrap()
        .order_intent()
        .is_some());

    let mut cancelled = Position::new(
        PositionId("cancelled-buy".to_string()),
        PositionKey {
            portfolio_id: strategy.config.portfolio_id.clone(),
            wallet_id: strategy.config.wallet_id.clone(),
            strategy_name: strategy.name(),
            token_address: pool.token_address,
            pool_address: pool.address.clone(),
            protocol: pool.protocol.clone(),
        },
    );
    cancelled.mark_intent_created(OrderSide::Buy).unwrap();
    cancelled
        .mark_order_submitted(OrderId("buy-1".to_string()), OrderSide::Buy)
        .unwrap();
    cancelled
        .apply_execution_report(&ExecutionReport {
            order_id: OrderId("buy-1".to_string()),
            status: ExecutionStatus::Cancelled,
            tx_hash: None,
            block_number: Some(1),
            filled_amount: None,
            token_amount: None,
            gas_used: None,
            gas_cost: None,
            mined_evidence: None,
            error: Some("simulation reverted before broadcast".to_string()),
        })
        .unwrap();

    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(cancelled.id.clone(), cancelled);
    let retry_ctx = ctx(&market, &portfolio, &risks);
    let retry = strategy
        .on_market_event(
            &retry_ctx,
            &MarketEvent::PoolUpdated {
                block_number: 2,
                pool,
            },
        )
        .unwrap();

    assert!(retry.order_intent().is_some());
    assert_eq!(
        retry.reason(),
        Some("entry.buy_eligible_pool_once:retry_after_noncapital_execution")
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

#[test]
fn entry_bankroll_restores_closed_profit_without_portfolio_position() {
    let pool = pool();
    let market = MarketSnapshotRef {
        block_number: 3,
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        token: None,
        pool: Some(pool.clone()),
    };
    let risks = Vec::new();
    let portfolio = PortfolioState::default();
    let ctx = ctx(&market, &portfolio, &risks);

    let mut closed_pool = pool.clone();
    closed_pool.token_address = Address::repeat_byte(0xcc);
    closed_pool.address = TokenPoolId::new(
        closed_pool.token_address,
        Address::repeat_byte(0xdd).to_string(),
    );

    let mut restored_bankroll = RestoredEntryBankroll::default();
    restored_bankroll.record_position_result(
        closed_pool.address.clone(),
        U256::from(10_000_000_000_000_000u64),
        U256::from(20_000_000_000_000_000u64),
    );
    let mut strategy = SnipeAllStrategy::with_restored_runtime_state(
        SnipeAllConfig {
            entry_bankroll_wei: Some(U256::from(10_000_000_000_000_000u64)),
            ..SnipeAllConfig::default()
        },
        vec![closed_pool.address],
        Vec::new(),
        restored_bankroll,
    );

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

fn trading_enabled_risk(pool: &PoolSnapshot, evidence: Option<Value>) -> RiskEvent {
    RiskEvent {
        kind: RiskKind::TradingEnabled,
        severity: RiskSeverity::Info,
        source: Some(RISK_SOURCE_MEMPOOL_SIGNAL.to_string()),
        token_address: pool.token_address,
        pool_address: Some(pool.address.clone()),
        pending_tx_hash: None,
        observed_block: Some(2),
        message: "Trading enabled".to_string(),
        evidence: evidence.map(|entry_evidence| {
            let mut evidence = serde_json::Map::new();
            evidence.insert(MEMPOOL_ENTRY_EVIDENCE_KEY.to_string(), entry_evidence);
            Value::Object(evidence)
        }),
    }
}

fn mempool_entry_evidence(pool: &PoolSnapshot, price_ratio_to_initial: Option<Decimal>) -> Value {
    let price_ratio_to_initial = price_ratio_to_initial.map(|value| value.to_string());
    json!({
        "evidence_version": MEMPOOL_ENTRY_EVIDENCE_VERSION,
        "base_block": 2,
        "simulated_block": 2,
        "dependency_tx_hashes": [
            "0x3333333333333333333333333333333333333333333333333333333333333333"
        ],
        "dependency_fee_metadata": {
            "tail_after_tx_hash": "0x3333333333333333333333333333333333333333333333333333333333333333"
        },
        "projected_pool": {
            "protocol": "UNISWAP-V2",
            "denom_address": pool
                .denom_address
                .map(|address| format!("{address:#x}")),
            "denom_symbol": pool.denom_symbol.clone(),
            "denom_reserve": "1",
            "token_reserve": "100",
            "price_ratio_to_initial": price_ratio_to_initial,
            "pool_creation_block": 1,
            "latest_block": 2,
            "can_buy": true,
            "can_sell": true,
            "is_scam": false
        },
        "viability": {
            "can_buy": true,
            "can_approve": true,
            "can_sell": true,
            "buy_tax_percent": "0",
            "sell_tax_percent": "0"
        },
        "vault_buy_simulation": {
            "route": "uniswap_v2_trading_vault",
            "would_revert": false,
            "gas_used": 176000,
            "eth_spent_wei": "10000000000000000",
            "tokens_received_raw": "1000000",
            "metadata": {
                "exact_vault_calldata": true
            }
        },
        "strategy_neutral_flags": {},
        "audit": {
            "source": "unit_test"
        }
    })
}
