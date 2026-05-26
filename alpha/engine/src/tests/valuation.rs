use super::*;

#[test]
fn submitted_exits_are_not_marked_to_market() {
    assert!(should_snapshot_position_for_pool(&test_position(
        PositionState::BuyConfirmed
    )));
    assert!(!should_snapshot_position_for_pool(&test_position(
        PositionState::SellSubmitted
    )));
    assert!(should_snapshot_position_for_pool(&test_position(
        PositionState::SellFailed
    )));
}

#[test]
fn drained_closed_positions_are_not_resnapshotted() {
    let mut open_drained = test_position(PositionState::BuyConfirmed);
    open_drained.drained = true;
    assert!(should_snapshot_position_for_pool(&open_drained));

    let mut pending_sell_drained = test_position(PositionState::SellSubmitted);
    pending_sell_drained.drained = true;
    assert!(should_snapshot_position_for_pool(&pending_sell_drained));

    let mut closed_drained = test_position(PositionState::SellConfirmed);
    closed_drained.drained = true;
    assert!(!should_snapshot_position_for_pool(&closed_drained));
}

#[test]
fn zero_value_snapshot_for_closed_position_has_no_unrealized_pnl() {
    let mut position = test_position(PositionState::SellConfirmed);
    position.entry_cost_basis = Some(DecimalAmount::from_str_exact("0.01").unwrap());
    position.exit_proceeds = Some(DecimalAmount::from_str_exact("0.02").unwrap());

    let snapshot = zero_value_snapshot(&position, 10, None);

    assert_eq!(snapshot.current_value_eth, DecimalAmount::ZERO);
    assert_eq!(snapshot.unrealized_profit_eth, DecimalAmount::ZERO);
    assert_eq!(snapshot.realized_profit_eth.to_string(), "0.01");
    assert_eq!(snapshot.roi.to_string(), "1");
}

#[test]
fn zero_current_value_snapshot_does_not_copy_pool_metrics() {
    let mut position = test_position(PositionState::BuyConfirmed);
    position.entry_cost_basis = Some(DecimalAmount::from_str_exact("0.01").unwrap());
    let token = position.key.token_address;
    let pool_address = Address::repeat_byte(0x22);
    let mut pool = pool_snapshot(token, pool_address, 10);
    pool.denom_reserve = DecimalAmount::from_str_exact("1.5").unwrap();
    pool.token_reserve = DecimalAmount::from_str_exact("1000000").unwrap();
    pool.price_ratio_to_initial = Some(DecimalAmount::from_str_exact("2").unwrap());
    pool.price_denom_per_token = Some(DecimalAmount::from_str_exact("0.000000002").unwrap());
    pool.initial_price_denom_per_token =
        Some(DecimalAmount::from_str_exact("0.000000001").unwrap());

    let snapshot = simulated_value_snapshot(
        &position,
        PositionValueSimulation {
            block_number: 10,
            current_value: Amount::zero(18),
            gas_used: None,
            error: None,
        },
        Some(&pool),
    );

    assert_eq!(snapshot.current_value_eth, DecimalAmount::ZERO);
    assert_eq!(snapshot.pool_liquidity_denom, None);
    assert_eq!(snapshot.pool_price_to_initial_price_ratio, None);
    assert_eq!(snapshot.pool_price_denom_per_token, None);
}

#[test]
fn display_zero_current_value_snapshot_does_not_copy_pool_metrics() {
    let mut position = test_position(PositionState::BuyConfirmed);
    position.entry_cost_basis = Some(DecimalAmount::from_str_exact("0.01").unwrap());
    let token = position.key.token_address;
    let pool_address = Address::repeat_byte(0x22);
    let mut pool = pool_snapshot(token, pool_address, 10);
    pool.denom_reserve = DecimalAmount::from_str_exact("1.5").unwrap();
    pool.token_reserve = DecimalAmount::from_str_exact("1000000").unwrap();
    pool.price_ratio_to_initial = Some(DecimalAmount::from_str_exact("2").unwrap());
    pool.price_denom_per_token = Some(DecimalAmount::from_str_exact("0.000000002").unwrap());
    pool.initial_price_denom_per_token =
        Some(DecimalAmount::from_str_exact("0.000000001").unwrap());

    let snapshot = simulated_value_snapshot(
        &position,
        PositionValueSimulation {
            block_number: 10,
            current_value: Amount {
                raw: U256::from(61_874_621_213u64),
                decimals: 18,
            },
            gas_used: None,
            error: None,
        },
        Some(&pool),
    );

    assert_eq!(
        snapshot.current_value_eth.to_string(),
        "0.000000061874621213"
    );
    assert_eq!(snapshot.pool_liquidity_denom, None);
    assert_eq!(snapshot.pool_price_to_initial_price_ratio, None);
    assert_eq!(snapshot.pool_price_denom_per_token, None);
}

#[tokio::test]
async fn newer_observed_snapshot_refines_same_valuation_coordinate() {
    let store = MemoryTradingStore::default();
    let mut engine = AlphaEngine::new(AllowAllRiskPolicy, store.clone(), NoopTestExecutionAdapter);
    let position = test_position(PositionState::BuyConfirmed);
    let mut snapshot = PositionSnapshot {
        position_id: position.id.clone(),
        trade_id: position.trade_id.clone(),
        state: PositionState::BuyConfirmed,
        block_number: 12,
        observed_block_number: Some(10),
        valuation_block_number: Some(12),
        current_value_eth: DecimalAmount::from_str_exact("0.01").unwrap(),
        realized_profit_eth: DecimalAmount::ZERO,
        unrealized_profit_eth: DecimalAmount::ZERO,
        roi: DecimalAmount::ZERO,
        pool_price_to_initial_price_ratio: Some(DecimalAmount::from_str_exact("1").unwrap()),
        pool_initial_price_denom_per_token: Some(
            DecimalAmount::from_str_exact("0.000000001").unwrap(),
        ),
        pool_price_denom_per_token: Some(DecimalAmount::from_str_exact("0.000000001").unwrap()),
        pool_liquidity_denom: Some(DecimalAmount::from_str_exact("1.2").unwrap()),
        pool_token_reserve: Some(DecimalAmount::from_str_exact("1000000").unwrap()),
        pool_denom_symbol: Some("WETH".to_string()),
    };
    engine
        .append_position_snapshot_once(snapshot.clone())
        .await
        .unwrap();

    snapshot.observed_block_number = Some(12);
    snapshot.pool_price_to_initial_price_ratio = Some(DecimalAmount::ZERO);
    snapshot.pool_price_denom_per_token = Some(DecimalAmount::ZERO);
    snapshot.pool_liquidity_denom = Some(DecimalAmount::from_str_exact("0.000000061875").unwrap());
    engine
        .append_position_snapshot_once(snapshot)
        .await
        .unwrap();

    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].observed_block_number, Some(12));
    assert_eq!(
        snapshots[0].pool_liquidity_denom.unwrap().to_string(),
        "0.000000061875"
    );
}

#[tokio::test]
async fn cancelled_sell_does_not_write_zero_value_snapshot() {
    let store = MemoryTradingStore::default();
    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    let key = PositionKey {
        portfolio_id: PortfolioId("chain-sim".to_string()),
        wallet_id: WalletId("chain-sim-wallet".to_string()),
        strategy_name: StrategyName("monitor-exit".to_string()),
        token_address: token,
        pool_address: TokenPoolId::new(token, pool_address.to_string()),
        protocol: PoolProtocol::UniswapV2,
    };
    let mut position = Position::new(position_id_for_key(&key), key);
    position.state = PositionState::BuyConfirmed;
    position.entry_cost_basis = Some(DecimalAmount::from_str_exact("0.01").unwrap());
    position.entry_token_raw_amount = Some(Amount {
        raw: U256::from(1_000_000u64),
        decimals: 18,
    });

    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);

    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        CancelledSellWithValueAdapter,
    )
    .with_portfolio(portfolio);
    engine.add_strategy(Box::new(MonitorExitStrategy));

    let reports = engine
        .handle_event(EngineEvent::Market(MarketEvent::BlockCompleted {
            block_number: 2,
            updated_tokens: 0,
            updated_pools: 0,
        }))
        .await
        .unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, ExecutionStatus::Cancelled);
    assert_eq!(store.positions()[0].state, PositionState::SellCancelled);
    assert!(store.snapshots().is_empty());
}

#[tokio::test]
async fn cancelled_sell_is_valued_on_next_pool_update() {
    let store = MemoryTradingStore::default();
    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    let key = PositionKey {
        portfolio_id: PortfolioId("chain-sim".to_string()),
        wallet_id: WalletId("chain-sim-wallet".to_string()),
        strategy_name: StrategyName("monitor-exit".to_string()),
        token_address: token,
        pool_address: TokenPoolId::new(token, pool_address.to_string()),
        protocol: PoolProtocol::UniswapV2,
    };
    let mut position = Position::new(position_id_for_key(&key), key);
    position.state = PositionState::BuyConfirmed;
    position.entry_cost_basis = Some(DecimalAmount::from_str_exact("0.01").unwrap());
    position.entry_token_raw_amount = Some(Amount {
        raw: U256::from(1_000_000u64),
        decimals: 18,
    });

    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);

    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        CancelledSellWithValueAdapter,
    )
    .with_portfolio(portfolio);
    engine.add_strategy(Box::new(MonitorExitStrategy));

    engine
        .handle_event(EngineEvent::Market(MarketEvent::BlockCompleted {
            block_number: 2,
            updated_tokens: 0,
            updated_pools: 0,
        }))
        .await
        .unwrap();
    engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 3,
            pool: pool_snapshot(token, pool_address, 3),
        }))
        .await
        .unwrap();

    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].state, PositionState::SellCancelled);
    assert_eq!(snapshots[0].block_number, 3);
    assert_eq!(
        snapshots[0].current_value_eth,
        DecimalAmount::from_str_exact("0.005").unwrap()
    );
    assert_eq!(
        snapshots[0].unrealized_profit_eth,
        DecimalAmount::from_str_exact("-0.005").unwrap()
    );
}

#[tokio::test]
async fn liquidity_removal_snapshot_does_not_copy_stale_pool_metrics() {
    let store = MemoryTradingStore::default();
    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    let pool = TokenPoolId::new(token, pool_address.to_string());
    let key = PositionKey {
        portfolio_id: PortfolioId("chain-sim".to_string()),
        wallet_id: WalletId("chain-sim-wallet".to_string()),
        strategy_name: StrategyName("strategy".to_string()),
        token_address: token,
        pool_address: pool.clone(),
        protocol: PoolProtocol::UniswapV2,
    };
    let mut position = Position::new(position_id_for_key(&key), key);
    position.state = PositionState::BuyConfirmed;
    position.entry_cost_basis = Some(DecimalAmount::from_str_exact("0.01").unwrap());

    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);

    let mut engine = AlphaEngine::new(AllowAllRiskPolicy, store.clone(), NoopTestExecutionAdapter)
        .with_portfolio(portfolio);
    let mut old_pool = pool_snapshot(token, pool_address, 1);
    old_pool.denom_reserve = DecimalAmount::from(1);
    engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 1,
            pool: old_pool,
        }))
        .await
        .unwrap();
    engine
        .handle_event(EngineEvent::Risk(RiskEvent {
            kind: RiskKind::LiquidityRemoval,
            severity: RiskSeverity::Critical,
            source: None,
            token_address: token,
            pool_address: Some(pool),
            pending_tx_hash: None,
            observed_block: Some(10),
            message: "liquidity removal".to_string(),
            evidence: None,
        }))
        .await
        .unwrap();

    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].block_number, 10);
    assert_eq!(snapshots[0].observed_block_number, Some(10));
    assert_eq!(snapshots[0].current_value_eth, DecimalAmount::ZERO);
    assert_eq!(snapshots[0].pool_liquidity_denom, None);
    assert_eq!(snapshots[0].pool_price_to_initial_price_ratio, None);
}
