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

#[path = "tests/valuation.rs"]
mod valuation;

#[path = "tests/lifecycle.rs"]
mod lifecycle;
