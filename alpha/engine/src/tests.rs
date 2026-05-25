use alloy_primitives::{Address, U256};
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, PortfolioId, StrategyName, TokenPoolId, WalletId},
    market::{PoolProtocol, PoolSnapshot},
    order::{OrderIntent, OrderSide},
    position::{Position, PositionKey, PositionSnapshot, PositionState},
    risk::{RiskKind, RiskSeverity},
    strategy::{StrategyContext, StrategyDecision},
    Result, Strategy,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

use super::*;
use crate::valuation::{
    should_snapshot_position_for_pool, simulated_value_snapshot, zero_value_snapshot,
};

#[path = "test_support.rs"]
mod test_support;
use test_support::*;

static TEST_POSITION_COUNTER: AtomicU64 = AtomicU64::new(1);

fn position_id_for_key(key: &PositionKey) -> eth_alpha_core::ids::PositionId {
    let seed = TEST_POSITION_COUNTER.fetch_add(1, Ordering::Relaxed);
    eth_alpha_core::ids::PositionId(format!(
        "legacy_{}_{}_{}_{}",
        sanitize_id_part(&key.strategy_name.0),
        key.token_address,
        key.pool_address,
        seed
    ))
}

fn sanitize_id_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

struct BuyOnMarketStrategy;

impl Strategy for BuyOnMarketStrategy {
    fn name(&self) -> StrategyName {
        StrategyName("buy-on-market".to_string())
    }

    fn on_market_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        _event: &MarketEvent,
    ) -> Result<StrategyDecision> {
        let pool = ctx.market.pool.as_ref().expect("pool snapshot");
        Ok(StrategyDecision::SubmitOrder(OrderIntent {
            trade_id: None,
            portfolio_id: PortfolioId("chain-sim".to_string()),
            wallet_id: WalletId("chain-sim-wallet".to_string()),
            strategy_name: self.name(),
            side: OrderSide::Buy,
            token_address: pool.token_address,
            pool_address: pool.address.clone(),
            protocol: pool.protocol.clone(),
            amount: Amount {
                raw: U256::from(1_000_000u64),
                decimals: 18,
            },
            route: None,
            max_slippage_bps: 500,
            deadline_secs: 30,
            decision_reason: None,
        }))
    }
}

struct BuyOnceThenHoldStrategy;

impl Strategy for BuyOnceThenHoldStrategy {
    fn name(&self) -> StrategyName {
        StrategyName("buy-once-then-hold".to_string())
    }

    fn on_market_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        _event: &MarketEvent,
    ) -> Result<StrategyDecision> {
        let pool = ctx.market.pool.as_ref().expect("pool snapshot");
        let has_position = ctx.portfolio.positions.values().any(|position| {
            position.key.strategy_name == self.name() && position.key.pool_address == pool.address
        });
        if has_position {
            return Ok(StrategyDecision::hold("position_open"));
        }

        Ok(StrategyDecision::SubmitOrder(OrderIntent {
            trade_id: None,
            portfolio_id: PortfolioId("chain-sim".to_string()),
            wallet_id: WalletId("chain-sim-wallet".to_string()),
            strategy_name: self.name(),
            side: OrderSide::Buy,
            token_address: pool.token_address,
            pool_address: pool.address.clone(),
            protocol: pool.protocol.clone(),
            amount: Amount {
                raw: U256::from(1_000_000u64),
                decimals: 18,
            },
            route: None,
            max_slippage_bps: 500,
            deadline_secs: 30,
            decision_reason: None,
        }))
    }
}

struct BuyThenSellWhenConfirmedStrategy;

impl Strategy for BuyThenSellWhenConfirmedStrategy {
    fn name(&self) -> StrategyName {
        StrategyName("buy-then-sell-when-confirmed".to_string())
    }

    fn on_market_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        _event: &MarketEvent,
    ) -> Result<StrategyDecision> {
        let pool = ctx.market.pool.as_ref().expect("pool snapshot");
        let position = ctx.portfolio.positions.values().find(|position| {
            position.key.strategy_name == self.name() && position.key.pool_address == pool.address
        });
        if let Some(position) = position {
            if position.state == PositionState::BuyConfirmed {
                let amount = position.entry_token_raw_amount.clone().unwrap_or(Amount {
                    raw: U256::from(1_000_000u64),
                    decimals: 18,
                });
                return Ok(StrategyDecision::submit_order(
                    OrderIntent {
                        trade_id: None,
                        portfolio_id: PortfolioId("chain-sim".to_string()),
                        wallet_id: WalletId("chain-sim-wallet".to_string()),
                        strategy_name: self.name(),
                        side: OrderSide::Sell,
                        token_address: pool.token_address,
                        pool_address: pool.address.clone(),
                        protocol: pool.protocol.clone(),
                        amount,
                        route: None,
                        max_slippage_bps: 500,
                        deadline_secs: 30,
                        decision_reason: None,
                    },
                    "exit.buy_confirmed",
                ));
            }
            return Ok(StrategyDecision::hold("position_not_buy_confirmed"));
        }

        Ok(StrategyDecision::submit_order(
            OrderIntent {
                trade_id: None,
                portfolio_id: PortfolioId("chain-sim".to_string()),
                wallet_id: WalletId("chain-sim-wallet".to_string()),
                strategy_name: self.name(),
                side: OrderSide::Buy,
                token_address: pool.token_address,
                pool_address: pool.address.clone(),
                protocol: pool.protocol.clone(),
                amount: Amount {
                    raw: U256::from(1_000_000u64),
                    decimals: 18,
                },
                route: None,
                max_slippage_bps: 500,
                deadline_secs: 30,
                decision_reason: None,
            },
            "entry.first_pool_update",
        ))
    }
}

struct BuyThenRiskSellWhenConfirmedStrategy;

impl Strategy for BuyThenRiskSellWhenConfirmedStrategy {
    fn name(&self) -> StrategyName {
        StrategyName("buy-then-risk-sell-when-confirmed".to_string())
    }

    fn on_market_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        _event: &MarketEvent,
    ) -> Result<StrategyDecision> {
        let Some(pool) = ctx.market.pool.as_ref() else {
            return Ok(StrategyDecision::hold("market.no_pool"));
        };
        let has_position = ctx.portfolio.positions.values().any(|position| {
            position.key.strategy_name == self.name()
                && position.key.pool_address == pool.address
                && position.has_exposure()
        });
        if has_position {
            return Ok(StrategyDecision::hold("position_open"));
        }

        Ok(StrategyDecision::submit_order(
            OrderIntent {
                trade_id: None,
                portfolio_id: PortfolioId("chain-sim".to_string()),
                wallet_id: WalletId("chain-sim-wallet".to_string()),
                strategy_name: self.name(),
                side: OrderSide::Buy,
                token_address: pool.token_address,
                pool_address: pool.address.clone(),
                protocol: pool.protocol.clone(),
                amount: Amount {
                    raw: U256::from(1_000_000u64),
                    decimals: 18,
                },
                route: None,
                max_slippage_bps: 500,
                deadline_secs: 30,
                decision_reason: None,
            },
            "entry.first_pool_update",
        ))
    }

    fn on_risk_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        event: &RiskEvent,
    ) -> Result<StrategyDecision> {
        let position = ctx.portfolio.positions.values().find(|position| {
            position.key.strategy_name == self.name()
                && position.key.token_address == event.token_address
                && event
                    .pool_address
                    .as_ref()
                    .map(|pool| *pool == position.key.pool_address)
                    .unwrap_or(true)
                && position.state == PositionState::BuyConfirmed
        });
        let Some(position) = position else {
            return Ok(StrategyDecision::hold("risk.no_sellable_position"));
        };

        Ok(StrategyDecision::submit_order(
            OrderIntent {
                trade_id: None,
                portfolio_id: position.key.portfolio_id.clone(),
                wallet_id: position.key.wallet_id.clone(),
                strategy_name: self.name(),
                side: OrderSide::Sell,
                token_address: position.key.token_address,
                pool_address: position.key.pool_address.clone(),
                protocol: position.key.protocol.clone(),
                amount: position.entry_token_raw_amount.clone().unwrap_or(Amount {
                    raw: U256::from(1_000_000u64),
                    decimals: 18,
                }),
                route: None,
                max_slippage_bps: 500,
                deadline_secs: 30,
                decision_reason: None,
            },
            "exit.risk",
        ))
    }
}

struct MonitorExitStrategy;

impl Strategy for MonitorExitStrategy {
    fn name(&self) -> StrategyName {
        StrategyName("monitor-exit".to_string())
    }

    fn on_market_event(
        &mut self,
        _ctx: &StrategyContext<'_>,
        _event: &MarketEvent,
    ) -> Result<StrategyDecision> {
        Ok(StrategyDecision::hold("market.noop"))
    }

    fn on_position_monitor(
        &mut self,
        ctx: &StrategyContext<'_>,
        _block_number: u64,
    ) -> Result<Vec<StrategyDecision>> {
        Ok(ctx
            .portfolio
            .positions
            .values()
            .filter(|position| {
                position.key.strategy_name == self.name()
                    && position.state == PositionState::BuyConfirmed
            })
            .map(|position| {
                StrategyDecision::submit_order(
                    OrderIntent {
                        trade_id: None,
                        portfolio_id: position.key.portfolio_id.clone(),
                        wallet_id: position.key.wallet_id.clone(),
                        strategy_name: self.name(),
                        side: OrderSide::Sell,
                        token_address: position.key.token_address,
                        pool_address: position.key.pool_address.clone(),
                        protocol: position.key.protocol.clone(),
                        amount: position.entry_token_raw_amount.clone().unwrap_or(Amount {
                            raw: U256::from(1_000_000u64),
                            decimals: 18,
                        }),
                        route: None,
                        max_slippage_bps: 500,
                        deadline_secs: 30,
                        decision_reason: None,
                    },
                    "exit.monitor",
                )
            })
            .collect())
    }
}

#[derive(Clone, Default)]
struct ConfirmingTestExecutionAdapter;

#[async_trait::async_trait]
impl EngineExecutionAdapter for ConfirmingTestExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        Ok(ExecutionReport {
            order_id: eth_alpha_core::ids::OrderId("test-order".to_string()),
            status: ExecutionStatus::Confirmed,
            tx_hash: None,
            block_number: Some(2),
            filled_amount: Some(intent.amount.clone()),
            token_amount: Some(intent.amount),
            gas_used: Some(21_000),
            gas_cost: Some(Amount {
                raw: U256::from(21_000_000u64),
                decimals: 18,
            }),
            mined_evidence: None,
            error: None,
        })
    }

    async fn simulate_position_value(
        &self,
        _position: &Position,
        _pool: &PoolSnapshot,
    ) -> Result<Option<PositionValueSimulation>> {
        Ok(Some(PositionValueSimulation {
            block_number: 2,
            current_value: Amount {
                raw: U256::from(20_000_000_000_000_000u64),
                decimals: 18,
            },
            gas_used: Some(21_000),
            error: None,
        }))
    }
}

#[derive(Clone, Default)]
struct DeferredTestExecutionAdapter;

#[async_trait::async_trait]
impl EngineExecutionAdapter for DeferredTestExecutionAdapter {
    async fn execute(&self, _intent: OrderIntent) -> Result<ExecutionReport> {
        Ok(ExecutionReport {
            order_id: OrderId("deferred-order".to_string()),
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
    }
}

#[derive(Clone, Default)]
struct SequencedDeferredExecutionAdapter {
    next_order: Arc<AtomicU64>,
}

#[async_trait::async_trait]
impl EngineExecutionAdapter for SequencedDeferredExecutionAdapter {
    async fn execute(&self, _intent: OrderIntent) -> Result<ExecutionReport> {
        let order_number = self.next_order.fetch_add(1, Ordering::Relaxed);
        Ok(ExecutionReport {
            order_id: OrderId(format!("deferred-order-{order_number}")),
            status: ExecutionStatus::Deferred,
            tx_hash: None,
            block_number: Some(order_number),
            filled_amount: None,
            token_amount: None,
            gas_used: None,
            gas_cost: None,
            mined_evidence: None,
            error: Some("simulation state not ready".to_string()),
        })
    }
}

#[derive(Clone, Default)]
struct CountingNextBlockValuationAdapter {
    valuation_calls: Arc<AtomicU64>,
}

#[async_trait::async_trait]
impl EngineExecutionAdapter for CountingNextBlockValuationAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        Ok(ExecutionReport {
            order_id: eth_alpha_core::ids::OrderId("counting-order".to_string()),
            status: ExecutionStatus::Confirmed,
            tx_hash: None,
            block_number: Some(2),
            filled_amount: Some(intent.amount.clone()),
            token_amount: Some(intent.amount),
            gas_used: Some(21_000),
            gas_cost: Some(Amount {
                raw: U256::from(21_000_000u64),
                decimals: 18,
            }),
            mined_evidence: None,
            error: None,
        })
    }

    async fn simulate_position_value(
        &self,
        _position: &Position,
        pool: &PoolSnapshot,
    ) -> Result<Option<PositionValueSimulation>> {
        self.valuation_calls.fetch_add(1, Ordering::Relaxed);
        Ok(Some(PositionValueSimulation {
            block_number: pool.latest_block,
            current_value: Amount {
                raw: U256::from(20_000_000_000_000_000u64),
                decimals: 18,
            },
            gas_used: Some(21_000),
            error: None,
        }))
    }
}

#[derive(Clone, Default)]
struct SequencedNextBlockExecutionAdapter {
    next_order: Arc<Mutex<u64>>,
}

#[async_trait::async_trait]
impl EngineExecutionAdapter for SequencedNextBlockExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let mut next_order = self.next_order.lock().expect("adapter lock");
        let sequence = *next_order;
        *next_order += 1;
        drop(next_order);

        Ok(ExecutionReport {
            order_id: OrderId(format!("test-order-{sequence}")),
            status: ExecutionStatus::Confirmed,
            tx_hash: None,
            block_number: Some(sequence + 2),
            filled_amount: Some(intent.amount.clone()),
            token_amount: (intent.side == OrderSide::Buy).then_some(intent.amount),
            gas_used: Some(21_000),
            gas_cost: Some(Amount {
                raw: U256::from(21_000_000u64),
                decimals: 18,
            }),
            mined_evidence: None,
            error: None,
        })
    }
}

fn pool_snapshot(token: Address, pool_address: Address, block_number: u64) -> PoolSnapshot {
    PoolSnapshot {
        address: TokenPoolId::new(token, pool_address.to_string()),
        token_address: token,
        protocol: PoolProtocol::UniswapV2,
        denom_address: Some(Address::repeat_byte(0x33)),
        denom_symbol: Some("WETH".to_string()),
        denom_reserve: Default::default(),
        token_reserve: Default::default(),
        price_denom_per_token: None,
        initial_price_denom_per_token: None,
        price_ratio_to_initial: None,
        creation_block: Some(block_number),
        token_decimals: None,
        fee_tier: None,
        uniswap_v4: None,
        latest_block: block_number,
        can_buy: true,
        can_sell: true,
        is_scam: false,
    }
}

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
    assert_eq!(decisions[0].event_source, "market");
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
