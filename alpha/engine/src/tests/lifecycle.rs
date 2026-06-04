use super::*;

#[tokio::test]
async fn deferred_execution_does_not_record_synthetic_submitted_report() {
    let store = MemoryTradingStore::default();
    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        DeferredTestExecutionAdapter,
    );
    engine.add_strategy(Box::new(BuyOnMarketStrategy));

    let token = Address::repeat_byte(0x11);
    let pool = pool_snapshot(token, Address::repeat_byte(0x22), 1);
    let reports = engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 1,
            pool,
        }))
        .await
        .unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, ExecutionStatus::Deferred);
    let execution_reports = store.execution_reports();
    assert_eq!(execution_reports.len(), 1);
    assert_eq!(execution_reports[0].status, ExecutionStatus::Deferred);
    assert_eq!(store.positions()[0].state, PositionState::BuyDeferred);
}

#[tokio::test]
async fn engine_executes_approved_strategy_order() {
    let store = MemoryTradingStore::default();
    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        ConfirmingTestExecutionAdapter,
    );
    engine.add_strategy(Box::new(BuyOnMarketStrategy));

    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    let pool = TokenPoolId::new(token, pool_address.to_string());
    let reports = engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 1,
            pool: PoolSnapshot {
                address: pool,
                token_address: token,
                protocol: PoolProtocol::UniswapV2,
                denom_address: Some(Address::repeat_byte(0x33)),
                denom_symbol: Some("WETH".to_string()),
                denom_reserve: Default::default(),
                token_reserve: Default::default(),
                price_denom_per_token: None,
                initial_price_denom_per_token: None,
                price_ratio_to_initial: None,
                creation_block: Some(1),
                token_decimals: None,
                fee_tier: None,
                uniswap_v4: None,
                latest_block: 1,
                can_buy: true,
                can_sell: true,
                is_scam: false,
            },
        }))
        .await
        .unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, ExecutionStatus::Submitted);
    assert_eq!(store.order_intents().len(), 1);
    assert_eq!(store.positions()[0].state, PositionState::BuySubmitted);

    let reports = engine.flush_pending_executions().await.unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, ExecutionStatus::Confirmed);

    let execution_reports = store.execution_reports();
    assert_eq!(execution_reports.len(), 2);
    assert_eq!(execution_reports[0].status, ExecutionStatus::Submitted);
    assert_eq!(execution_reports[0].block_number, Some(1));
    assert_eq!(execution_reports[1].status, ExecutionStatus::Confirmed);
    assert_eq!(execution_reports[1].block_number, Some(2));
    assert_eq!(store.positions().len(), 1);
    let decisions = store.strategy_decisions();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].event_source, "pool_update");
    assert_eq!(decisions[0].action, "submit_buy");
    assert_eq!(decisions[0].order_side, Some(OrderSide::Buy));
    assert_eq!(engine.portfolio().active_position_count(), 1);
}

#[tokio::test]
async fn repeated_buys_on_same_strategy_pool_get_distinct_trade_ids() {
    let store = MemoryTradingStore::default();
    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        SequencedNextBlockExecutionAdapter::default(),
    );
    engine.add_strategy(Box::new(BuyOnMarketStrategy));

    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 1,
            pool: pool_snapshot(token, pool_address, 1),
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

    let positions = store.positions();
    assert_eq!(positions.len(), 2);
    let trade_ids = positions
        .iter()
        .map(|position| position.trade_id.0.clone())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(trade_ids.len(), 2);
    assert!(trade_ids.iter().all(|id| id.starts_with("trd_")));
    assert_eq!(
        store
            .order_intents()
            .iter()
            .filter(|intent| intent.trade_id.is_some())
            .count(),
        2
    );
}

#[tokio::test]
async fn deferred_buy_retry_reuses_same_trade_id() {
    let store = MemoryTradingStore::default();
    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        SequencedDeferredExecutionAdapter::default(),
    );
    engine.add_strategy(Box::new(BuyOnMarketStrategy));

    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 1,
            pool: pool_snapshot(token, pool_address, 1),
        }))
        .await
        .unwrap();
    engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 2,
            pool: pool_snapshot(token, pool_address, 2),
        }))
        .await
        .unwrap();

    let positions = store.positions();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].state, PositionState::BuyDeferred);
    assert_eq!(
        positions[0].entry_order_id.as_ref().map(|id| id.0.as_str()),
        Some("deferred-order-1")
    );

    let intents = store.order_intents();
    assert_eq!(intents.len(), 2);
    assert_eq!(intents[0].trade_id, intents[1].trade_id);
    assert_eq!(intents[0].trade_id.as_ref(), Some(&positions[0].trade_id));

    let reports = store.execution_reports();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].order_id.0, "deferred-order-0");
    assert_eq!(reports[1].order_id.0, "deferred-order-1");
}

#[tokio::test]
async fn confirmed_buy_gets_value_snapshot_without_later_pool_update() {
    let store = MemoryTradingStore::default();
    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        ConfirmingTestExecutionAdapter,
    );
    engine.add_strategy(Box::new(BuyOnMarketStrategy));

    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 1,
            pool: pool_snapshot(token, pool_address, 1),
        }))
        .await
        .unwrap();

    let reports = engine.flush_pending_executions().await.unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, ExecutionStatus::Confirmed);

    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].state, PositionState::BuyConfirmed);
    assert_eq!(snapshots[0].block_number, 2);
    assert_eq!(
        snapshots[0].current_value_eth,
        DecimalAmount::from_str_exact("0.02").unwrap()
    );
}

#[tokio::test]
async fn pending_buy_confirmed_during_pool_update_uses_single_market_valuation() {
    let store = MemoryTradingStore::default();
    let adapter = CountingNextBlockValuationAdapter::default();
    let valuation_calls = adapter.valuation_calls.clone();
    let mut engine = AlphaEngine::new(AllowAllRiskPolicy, store.clone(), adapter);
    engine.add_strategy(Box::new(BuyOnceThenHoldStrategy));

    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 1,
            pool: pool_snapshot(token, pool_address, 1),
        }))
        .await
        .unwrap();
    engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 2,
            pool: pool_snapshot(token, pool_address, 2),
        }))
        .await
        .unwrap();

    let snapshots = store.snapshots();
    assert_eq!(valuation_calls.load(Ordering::Relaxed), 1);
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].state, PositionState::BuyConfirmed);
    assert_eq!(snapshots[0].block_number, 2);
}

#[tokio::test]
async fn pending_reports_apply_before_later_market_updates() {
    let store = MemoryTradingStore::default();
    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        SequencedNextBlockExecutionAdapter::default(),
    );
    engine.add_strategy(Box::new(BuyThenSellWhenConfirmedStrategy));

    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);

    let reports = engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 1,
            pool: pool_snapshot(token, pool_address, 1),
        }))
        .await
        .unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, ExecutionStatus::Submitted);
    assert_eq!(store.positions()[0].state, PositionState::BuySubmitted);

    let reports = engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 2,
            pool: pool_snapshot(token, pool_address, 2),
        }))
        .await
        .unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].status, ExecutionStatus::Confirmed);
    assert_eq!(reports[0].block_number, Some(2));
    assert_eq!(reports[1].status, ExecutionStatus::Submitted);
    assert_eq!(store.positions()[0].state, PositionState::SellSubmitted);

    let reports = engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 3,
            pool: pool_snapshot(token, pool_address, 3),
        }))
        .await
        .unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, ExecutionStatus::Confirmed);
    assert_eq!(reports[0].block_number, Some(3));
    assert_eq!(store.positions()[0].state, PositionState::SellConfirmed);

    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].state, PositionState::SellConfirmed);
    assert_eq!(snapshots[0].block_number, 3);
}

#[tokio::test]
async fn pending_reports_apply_before_same_block_risk_updates() {
    let store = MemoryTradingStore::default();
    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        SequencedNextBlockExecutionAdapter::default(),
    );
    engine.add_strategy(Box::new(BuyThenRiskSellWhenConfirmedStrategy));

    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    let pool = TokenPoolId::new(token, pool_address.to_string());

    let reports = engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 1,
            pool: pool_snapshot(token, pool_address, 1),
        }))
        .await
        .unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, ExecutionStatus::Submitted);
    assert_eq!(store.positions()[0].state, PositionState::BuySubmitted);

    let reports = engine
        .handle_event(EngineEvent::Risk(RiskEvent {
            kind: RiskKind::LpApproval,
            severity: RiskSeverity::Warning,
            source: None,
            token_address: token,
            pool_address: Some(pool),
            pending_tx_hash: None,
            observed_block: Some(2),
            message: "lp approval".to_string(),
            evidence: None,
        }))
        .await
        .unwrap();

    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].status, ExecutionStatus::Confirmed);
    assert_eq!(reports[0].block_number, Some(2));
    assert_eq!(reports[1].status, ExecutionStatus::Submitted);
    assert_eq!(reports[1].block_number, Some(2));
    assert_eq!(store.positions()[0].state, PositionState::SellSubmitted);

    let decisions = store.strategy_decisions();
    assert_eq!(decisions.len(), 2);
    assert_eq!(decisions[0].action, "submit_buy");
    assert_eq!(decisions[1].event_source, "risk");
    assert_eq!(decisions[1].action, "submit_sell");
    assert_eq!(decisions[1].reason_code.as_deref(), Some("exit.risk"));
    let sell_order_reason = store
        .order_intents()
        .into_iter()
        .find(|intent| intent.side == OrderSide::Sell)
        .and_then(|intent| intent.decision_reason);
    let sell_order_reason = sell_order_reason.expect("sell order decision rationale");
    assert_eq!(sell_order_reason.code, "exit.risk");
    assert_eq!(sell_order_reason.source.as_deref(), Some("risk"));
}

#[tokio::test]
async fn external_execution_report_updates_matching_submitted_position() {
    let store = MemoryTradingStore::default();
    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    let key = PositionKey {
        portfolio_id: PortfolioId("real".to_string()),
        wallet_id: WalletId("real-wallet".to_string()),
        strategy_name: StrategyName("real-receipt".to_string()),
        token_address: token,
        pool_address: TokenPoolId::new(token, pool_address.to_string()),
        protocol: PoolProtocol::UniswapV2,
    };
    let mut position = Position::new(position_id_for_key(&key), key);
    position.state = PositionState::BuySubmitted;
    position.entry_order_id = Some(OrderId("order-1".to_string()));

    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);
    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        ConfirmingTestExecutionAdapter,
    )
    .with_portfolio(portfolio);

    let reports = engine
        .handle_event(EngineEvent::Execution(ExecutionReport {
            order_id: OrderId("order-1".to_string()),
            status: ExecutionStatus::Confirmed,
            tx_hash: Some(
                "0x1111111111111111111111111111111111111111111111111111111111111111"
                    .parse()
                    .unwrap(),
            ),
            block_number: Some(100),
            filled_amount: Some(Amount {
                raw: U256::from(10u64),
                decimals: 18,
            }),
            token_amount: Some(Amount {
                raw: U256::from(20u64),
                decimals: 18,
            }),
            gas_used: Some(21_000),
            gas_cost: Some(Amount {
                raw: U256::from(21_000_000_000_000u64),
                decimals: 18,
            }),
            mined_evidence: None,
            error: None,
        }))
        .await
        .unwrap();

    assert_eq!(reports.len(), 1);
    assert_eq!(store.positions()[0].state, PositionState::BuyConfirmed);
    assert_eq!(store.positions()[0].entry_block, Some(100));
    assert_eq!(store.execution_reports().len(), 1);
    assert_eq!(
        store.execution_reports()[0].status,
        ExecutionStatus::Confirmed
    );
}

#[tokio::test]
async fn position_monitor_runs_without_prior_market_after_restore() {
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
    position.entry_token_raw_amount = Some(Amount {
        raw: U256::from(1_000_000u64),
        decimals: 18,
    });

    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);

    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        ConfirmingTestExecutionAdapter,
    )
    .with_portfolio(portfolio);
    engine.add_strategy(Box::new(MonitorExitStrategy));

    let reports = engine
        .handle_event(EngineEvent::Market(MarketEvent::BlockCompleted {
            block_number: 10,
            updated_tokens: 0,
            updated_pools: 0,
        }))
        .await
        .unwrap();

    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].status, ExecutionStatus::Submitted);
    assert_eq!(reports[1].status, ExecutionStatus::Confirmed);
}

#[tokio::test]
async fn default_risk_event_strategy_hook_holds() {
    let store = MemoryTradingStore::default();
    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        ConfirmingTestExecutionAdapter,
    );
    engine.add_strategy(Box::new(BuyOnMarketStrategy));

    let reports = engine
        .handle_event(EngineEvent::Risk(RiskEvent {
            kind: RiskKind::LpApproval,
            severity: RiskSeverity::Warning,
            source: None,
            token_address: Address::repeat_byte(0x33),
            pool_address: Some(TokenPoolId::new(
                Address::repeat_byte(0x33),
                Address::repeat_byte(0x44).to_string(),
            )),
            pending_tx_hash: None,
            observed_block: None,
            message: "lp approval".to_string(),
            evidence: None,
        }))
        .await
        .unwrap();

    assert!(reports.is_empty());
    assert_eq!(engine.active_risks().len(), 1);
    assert!(store.order_intents().is_empty());
    let decisions = store.strategy_decisions();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].event_source, "risk");
    assert_eq!(decisions[0].action, "hold");
}

#[tokio::test]
async fn critical_risk_policy_rejects_matching_order() {
    let store = MemoryTradingStore::default();
    let mut engine = AlphaEngine::new(
        BlockCriticalRiskPolicy,
        store.clone(),
        ConfirmingTestExecutionAdapter,
    );
    engine.add_strategy(Box::new(BuyOnMarketStrategy));

    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    let pool = TokenPoolId::new(token, pool_address.to_string());
    engine
        .handle_event(EngineEvent::Risk(RiskEvent {
            kind: RiskKind::LiquidityRemoval,
            severity: RiskSeverity::Critical,
            source: None,
            token_address: token,
            pool_address: Some(pool.clone()),
            pending_tx_hash: None,
            observed_block: Some(1),
            message: "liquidity removal".to_string(),
            evidence: None,
        }))
        .await
        .unwrap();

    let reports = engine
        .handle_event(EngineEvent::Market(MarketEvent::PoolUpdated {
            block_number: 1,
            pool: PoolSnapshot {
                address: pool,
                token_address: token,
                protocol: PoolProtocol::UniswapV2,
                denom_address: Some(Address::repeat_byte(0x33)),
                denom_symbol: Some("WETH".to_string()),
                denom_reserve: Default::default(),
                token_reserve: Default::default(),
                price_denom_per_token: None,
                initial_price_denom_per_token: None,
                price_ratio_to_initial: None,
                creation_block: Some(1),
                token_decimals: None,
                fee_tier: None,
                uniswap_v4: None,
                latest_block: 1,
                can_buy: true,
                can_sell: true,
                is_scam: false,
            },
        }))
        .await
        .unwrap();

    assert!(reports.is_empty());
    assert_eq!(store.order_intents().len(), 1);
    assert!(store.execution_reports().is_empty());
    assert!(store.positions().is_empty());
    assert_eq!(store.strategy_decisions().len(), 2);
}

// A mined liquidity-removal / scam drain must terminalize an open position to a
// zero-value `TerminalZero` close even when the strategy has no exit rule enabled
// (config-independent). A confiscated balance cannot be sold, so the close must
// not depend on a successful sell.
#[tokio::test]
async fn drained_position_terminalizes_to_terminal_zero_on_mined_liquidity_removal() {
    let store = MemoryTradingStore::default();
    let token = Address::repeat_byte(0x11);
    let pool_address = Address::repeat_byte(0x22);
    let pool = TokenPoolId::new(token, pool_address.to_string());

    let mut position = test_position(PositionState::BuyConfirmed);
    position.entry_cost_basis = Some(DecimalAmount::from_str_exact("0.005").unwrap());
    position.entry_block = Some(10);
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);

    // No strategy added on purpose: the terminal close is an engine-level
    // invariant, not gated by any `exit_*` strategy flag.
    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        ConfirmingTestExecutionAdapter,
    )
    .with_portfolio(portfolio);

    engine
        .handle_event(EngineEvent::Risk(RiskEvent {
            kind: RiskKind::LiquidityRemoval,
            severity: RiskSeverity::Critical,
            source: None,
            token_address: token,
            pool_address: Some(pool),
            pending_tx_hash: None,
            observed_block: Some(11),
            message: "holder-balance backdoor drain".to_string(),
            evidence: None,
        }))
        .await
        .unwrap();

    let position = store.positions().into_iter().next().expect("position");
    assert_eq!(position.state, PositionState::TerminalZero);
    assert!(position.drained);
    assert!(!position.has_exposure());

    // A terminal zero-value snapshot must back the drained close, and there must
    // be no positive valuation after it.
    let snapshots = store.snapshots();
    assert!(!snapshots.is_empty());
    assert!(snapshots
        .iter()
        .all(|snapshot| snapshot.current_value_eth == DecimalAmount::ZERO));
}

// A drained position whose sell fails (a confiscated balance cannot fill) must
// still terminalize to `TerminalZero` rather than lingering in `SellFailed`.
#[tokio::test]
async fn failed_sell_of_drained_position_terminalizes_to_terminal_zero() {
    let store = MemoryTradingStore::default();

    let mut position = test_position(PositionState::SellSubmitted);
    position.drained = true;
    position.exit_order_id = Some(OrderId("sell-1".to_string()));
    position.entry_cost_basis = Some(DecimalAmount::from_str_exact("0.005").unwrap());
    let mut portfolio = PortfolioState::default();
    portfolio.positions.insert(position.id.clone(), position);

    let mut engine = AlphaEngine::new(
        AllowAllRiskPolicy,
        store.clone(),
        ConfirmingTestExecutionAdapter,
    )
    .with_portfolio(portfolio);

    engine
        .handle_event(EngineEvent::Execution(ExecutionReport {
            order_id: OrderId("sell-1".to_string()),
            status: ExecutionStatus::Failed,
            tx_hash: None,
            block_number: Some(12),
            filled_amount: None,
            token_amount: None,
            gas_used: None,
            gas_cost: None,
            mined_evidence: None,
            error: Some("uneconomic sell".to_string()),
        }))
        .await
        .unwrap();

    let position = store.positions().into_iter().next().expect("position");
    assert_eq!(position.state, PositionState::TerminalZero);
    assert!(!position.has_exposure());
}
