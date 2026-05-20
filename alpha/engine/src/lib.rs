//! High-level alpha runtime skeleton.
//!
//! The engine sits above live feed/state and mempool risk. It consumes typed
//! market, risk, and execution events, runs strategies, applies risk policy, and
//! routes approved intents to an execution adapter. The no-capital runtime uses
//! chain-state EVM simulation; real submission remains isolated behind a future
//! `tx_executor` adapter.

pub mod execution;
pub mod wire;

mod decision_record;
mod event_flow;
mod ids;
mod memory_store;
mod policy;
mod snapshots;

use std::collections::HashMap;

use async_trait::async_trait;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    error::{AlphaCoreError, Result},
    execution::{ExecutionReport, ExecutionStatus},
    ids::{PositionId, TokenPoolId},
    market::{MarketEvent, MarketSnapshotRef, PoolSnapshot},
    order::{OrderIntent, OrderSide},
    portfolio::PortfolioState,
    position::{Position, PositionKey, PositionSnapshot, PositionState},
    risk::{RiskDecision, RiskEvent, RiskKind, RiskPolicy},
    store::TradingStore,
    strategy::{Strategy, StrategyContext, StrategyDecision},
};

// Re-export chain-simulation adapters at crate root for convenience.
pub use execution::{
    ChainSimExecutionAdapter, LiveChainSimExecutionAdapter, LiveTradingPlannerBridge,
    LiveTxPlanningInputResolver, TxExecutorAdapter,
};
pub use memory_store::MemoryTradingStore;
pub use policy::{AllowAllRiskPolicy, BlockCriticalRiskPolicy};

use decision_record::{risk_kind_key, strategy_decision_action, strategy_decision_record};
use event_flow::{
    engine_event_block, fill_price_for_report, market_event_block, should_defer_report,
    submitted_report_for,
};
use ids::new_trade_id;
use snapshots::{
    should_snapshot_position_for_pool, simulated_value_snapshot, snapshot_with_pool_metrics,
    valuation_safe_pool, zero_value_snapshot,
};

#[derive(Clone, Debug, PartialEq)]
pub enum EngineEvent {
    Market(MarketEvent),
    Risk(RiskEvent),
    Execution(ExecutionReport),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PositionValueSimulation {
    pub block_number: u64,
    pub current_value: Amount,
    pub gas_used: Option<u64>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct PendingExecutionReport {
    position_id: PositionId,
    side: OrderSide,
    report: ExecutionReport,
}

#[async_trait]
pub trait EngineExecutionAdapter: Send + Sync {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport>;

    async fn simulate_position_value(
        &self,
        _position: &Position,
        _pool: &PoolSnapshot,
    ) -> Result<Option<PositionValueSimulation>> {
        Ok(None)
    }
}

#[async_trait]
impl<T> EngineExecutionAdapter for Box<T>
where
    T: EngineExecutionAdapter + ?Sized,
{
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        (**self).execute(intent).await
    }

    async fn simulate_position_value(
        &self,
        position: &Position,
        pool: &PoolSnapshot,
    ) -> Result<Option<PositionValueSimulation>> {
        (**self).simulate_position_value(position, pool).await
    }
}

pub struct AlphaEngine<E, R, S>
where
    E: EngineExecutionAdapter,
    R: RiskPolicy,
    S: TradingStore,
{
    portfolio: PortfolioState,
    market: Option<MarketSnapshotRef>,
    active_risks: Vec<RiskEvent>,
    strategies: Vec<Box<dyn Strategy>>,
    risk_policy: R,
    store: S,
    execution: E,
    current_event_block: Option<u64>,
    pending_execution_reports: Vec<PendingExecutionReport>,
    pool_snapshots: HashMap<TokenPoolId, PoolSnapshot>,
}

impl<E, R, S> AlphaEngine<E, R, S>
where
    E: EngineExecutionAdapter,
    R: RiskPolicy,
    S: TradingStore,
{
    pub fn new(risk_policy: R, store: S, execution: E) -> Self {
        Self {
            portfolio: PortfolioState::default(),
            market: None,
            active_risks: Vec::new(),
            strategies: Vec::new(),
            risk_policy,
            store,
            execution,
            current_event_block: None,
            pending_execution_reports: Vec::new(),
            pool_snapshots: HashMap::new(),
        }
    }

    pub fn with_portfolio(mut self, portfolio: PortfolioState) -> Self {
        self.portfolio = portfolio;
        self
    }

    pub fn add_strategy(&mut self, strategy: Box<dyn Strategy>) {
        self.strategies.push(strategy);
    }

    pub fn portfolio(&self) -> &PortfolioState {
        &self.portfolio
    }

    pub fn active_risks(&self) -> &[RiskEvent] {
        &self.active_risks
    }

    pub fn market(&self) -> Option<&MarketSnapshotRef> {
        self.market.as_ref()
    }

    pub async fn handle_event(&mut self, event: EngineEvent) -> Result<Vec<ExecutionReport>> {
        self.current_event_block = engine_event_block(&event);
        match event {
            EngineEvent::Market(event) => {
                self.apply_market_event(&event);
                let mut reports = self
                    .apply_due_pending_execution_reports(market_event_block(&event))
                    .await?;
                reports.extend(self.run_market_strategies(&event).await?);
                Ok(reports)
            }
            EngineEvent::Risk(event) => {
                let mut reports = if let Some(block_number) = event.observed_block {
                    self.apply_due_pending_execution_reports(block_number)
                        .await?
                } else {
                    Vec::new()
                };
                self.active_risks.push(event.clone());
                self.store.record_risk_event(&event).await?;
                reports.extend(self.run_risk_strategies(&event).await?);
                Ok(reports)
            }
            EngineEvent::Execution(report) => {
                self.store.record_execution_report(&report).await?;
                Ok(vec![report])
            }
        }
    }

    pub async fn flush_pending_executions(&mut self) -> Result<Vec<ExecutionReport>> {
        let pending = std::mem::take(&mut self.pending_execution_reports);
        self.apply_pending_execution_reports(pending).await
    }

    fn apply_market_event(&mut self, event: &MarketEvent) {
        match event {
            MarketEvent::TokenUpdated {
                block_number,
                token,
            } => {
                self.market = Some(MarketSnapshotRef {
                    block_number: *block_number,
                    token_address: token.address,
                    pool_address: None,
                    token: Some(token.clone()),
                    pool: None,
                });
            }
            MarketEvent::PoolUpdated { block_number, pool } => {
                self.pool_snapshots
                    .insert(pool.address.clone(), pool.clone());
                self.market = Some(MarketSnapshotRef {
                    block_number: *block_number,
                    token_address: pool.token_address,
                    pool_address: Some(pool.address.clone()),
                    token: None,
                    pool: Some(pool.clone()),
                });
            }
            MarketEvent::BlockCompleted { block_number, .. } => {
                if let Some(market) = self.market.as_mut() {
                    market.block_number = *block_number;
                }
            }
        }
    }

    async fn run_market_strategies(&mut self, event: &MarketEvent) -> Result<Vec<ExecutionReport>> {
        let market = match self.market.clone() {
            Some(market) => market,
            None if matches!(event, MarketEvent::BlockCompleted { .. }) => MarketSnapshotRef {
                block_number: market_event_block(event),
                token_address: Default::default(),
                pool_address: None,
                token: None,
                pool: None,
            },
            None => return Ok(Vec::new()),
        };
        let portfolio = self.portfolio.clone();
        let active_risks = self.active_risks.clone();
        let ctx = StrategyContext {
            market: &market,
            portfolio: &portfolio,
            active_risks: &active_risks,
        };

        let mut market_decisions = Vec::with_capacity(self.strategies.len());
        let mut monitor_decisions = Vec::new();
        let mut market_decisions_to_record = Vec::new();
        let mut monitor_decisions_to_record = Vec::new();
        for strategy in &mut self.strategies {
            let strategy_name = strategy.name();
            let decision = strategy.on_market_event(&ctx, event)?;
            if !matches!(event, MarketEvent::BlockCompleted { .. }) {
                market_decisions_to_record.push((strategy_name.0.clone(), decision.clone()));
            }
            market_decisions.push(decision);
            if let MarketEvent::BlockCompleted { block_number, .. } = event {
                let decisions = strategy.on_position_monitor(&ctx, *block_number)?;
                for (index, decision) in decisions.iter().enumerate() {
                    monitor_decisions_to_record.push((
                        strategy_name.0.clone(),
                        *block_number,
                        index,
                        decision.clone(),
                    ));
                }
                monitor_decisions.extend(decisions);
            }
        }
        for (strategy_name, decision) in market_decisions_to_record {
            self.record_strategy_decision(&strategy_name, "market", event, &decision)
                .await?;
        }
        for (strategy_name, block_number, index, decision) in monitor_decisions_to_record {
            self.record_position_monitor_decision(&strategy_name, block_number, index, &decision)
                .await?;
        }
        let mut reports = self.apply_decisions(market_decisions, "market").await?;
        reports.extend(
            self.apply_decisions(monitor_decisions, "position_monitor")
                .await?,
        );
        self.snapshot_open_positions_for_pool(event).await?;
        Ok(reports)
    }

    async fn snapshot_open_positions_for_pool(&mut self, event: &MarketEvent) -> Result<()> {
        let MarketEvent::PoolUpdated { pool, block_number } = event else {
            return Ok(());
        };
        let positions = self
            .portfolio
            .positions
            .values()
            .filter(|position| {
                position.key.pool_address == pool.address
                    && should_snapshot_position_for_pool(position)
            })
            .cloned()
            .collect::<Vec<_>>();

        for position in positions {
            let snapshot = if position.drained {
                Some(zero_value_snapshot(&position, *block_number, Some(pool)))
            } else {
                self.execution
                    .simulate_position_value(&position, pool)
                    .await?
                    .map(|value| simulated_value_snapshot(&position, value, Some(pool)))
            };
            if let Some(snapshot) = snapshot {
                self.store.append_position_snapshot(&snapshot).await?;
            }
        }
        Ok(())
    }

    async fn run_risk_strategies(&mut self, event: &RiskEvent) -> Result<Vec<ExecutionReport>> {
        let cached_pool = event
            .pool_address
            .as_ref()
            .and_then(|pool_address| self.pool_snapshots.get(pool_address))
            .filter(|pool| pool.token_address == event.token_address)
            .cloned();
        let market = self
            .market
            .clone()
            .filter(|market| {
                market.token_address == event.token_address
                    && event
                        .pool_address
                        .as_ref()
                        .map(|pool| Some(pool) == market.pool_address.as_ref())
                        .unwrap_or(true)
            })
            .or_else(|| {
                cached_pool.as_ref().map(|pool| MarketSnapshotRef {
                    block_number: event.observed_block.unwrap_or(pool.latest_block),
                    token_address: event.token_address,
                    pool_address: Some(pool.address.clone()),
                    token: None,
                    pool: Some(pool.clone()),
                })
            })
            .unwrap_or_else(|| MarketSnapshotRef {
                block_number: event.observed_block.unwrap_or_default(),
                token_address: event.token_address,
                pool_address: event.pool_address.clone(),
                token: None,
                pool: None,
            });
        let portfolio = self.portfolio.clone();
        let active_risks = self.active_risks.clone();
        let ctx = StrategyContext {
            market: &market,
            portfolio: &portfolio,
            active_risks: &active_risks,
        };

        let mut decisions = Vec::with_capacity(self.strategies.len());
        let mut risk_decisions_to_record = Vec::new();
        for strategy in &mut self.strategies {
            let strategy_name = strategy.name();
            let decision = strategy.on_risk_event(&ctx, event)?;
            risk_decisions_to_record.push((strategy_name.0.clone(), decision.clone()));
            decisions.push(decision);
        }
        for (strategy_name, decision) in risk_decisions_to_record {
            self.record_risk_strategy_decision(&strategy_name, event, &decision)
                .await?;
        }
        let event_source = event.source.as_deref().unwrap_or("risk");
        let reports = self.apply_decisions(decisions, event_source).await?;

        // Worst-case baseline: mark open positions as drained on
        // liquidity removal or scam confirmation, even if strategy does not exit.
        if matches!(
            event.kind,
            RiskKind::LiquidityRemoval | RiskKind::ScamConfirmed
        ) {
            if let Some(ref pool_address) = event.pool_address {
                for position in self.portfolio.positions.values_mut() {
                    if position.key.pool_address == *pool_address
                        && position.has_exposure()
                        && !position.drained
                    {
                        position.mark_drained();
                        let _ = self.store.upsert_position(position).await;
                        // Snapshot the drained state so baseline PnL is honest
                        // even when no pool update follows the signal.
                        // Use a high block number so this snapshot is picked as
                        // the latest by DISTINCT ON ... ORDER BY block_number DESC.
                        let block_number = event.observed_block.unwrap_or(u64::MAX - 1);
                        let snapshot = PositionSnapshot {
                            position_id: position.id.clone(),
                            trade_id: position.trade_id.clone(),
                            state: position.state.clone(),
                            block_number,
                            observed_block_number: event.observed_block,
                            valuation_block_number: event.observed_block,
                            current_value_eth: DecimalAmount::ZERO,
                            realized_profit_eth: position.realized_pnl(),
                            unrealized_profit_eth: -position.entry_cost_basis.unwrap_or_default(),
                            roi: DecimalAmount::from(-1),
                            pool_price_to_initial_price_ratio: None,
                            pool_initial_price_denom_per_token: None,
                            pool_price_denom_per_token: None,
                            pool_liquidity_denom: None,
                            pool_token_reserve: None,
                            pool_denom_symbol: None,
                        };
                        let pool_snapshot = self.pool_snapshots.get(pool_address).filter(|pool| {
                            pool.token_address == event.token_address
                                && event
                                    .observed_block
                                    .map(|observed_block| pool.latest_block == observed_block)
                                    .unwrap_or(false)
                        });
                        let snapshot = snapshot_with_pool_metrics(snapshot, pool_snapshot);
                        let _ = self.store.append_position_snapshot(&snapshot).await;
                    }
                }
            }
        }

        Ok(reports)
    }

    async fn apply_decisions(
        &mut self,
        decisions: Vec<StrategyDecision>,
        event_source: &str,
    ) -> Result<Vec<ExecutionReport>> {
        let mut reports = Vec::new();
        for decision in decisions {
            let action = strategy_decision_action(&decision);
            let structured_reason = decision.structured_reason(Some(event_source), Some(action));
            match decision {
                StrategyDecision::Hold
                | StrategyDecision::HoldWithReason { .. }
                | StrategyDecision::CancelOrders { .. } => {}
                StrategyDecision::SubmitOrder(mut intent)
                | StrategyDecision::SubmitOrderWithReason { mut intent, .. } => {
                    intent.decision_reason = structured_reason;
                    reports.extend(self.execute_if_allowed(intent).await?);
                }
            }
        }
        Ok(reports)
    }

    async fn record_strategy_decision(
        &self,
        strategy_name: &str,
        event_source: &str,
        event: &MarketEvent,
        decision: &StrategyDecision,
    ) -> Result<()> {
        let (block_number, token_address, pool_address, event_key) = match event {
            MarketEvent::TokenUpdated {
                block_number,
                token,
            } => (
                Some(*block_number),
                Some(token.address.to_string()),
                None,
                format!("token:{}:{block_number}", token.address),
            ),
            MarketEvent::PoolUpdated { block_number, pool } => (
                Some(*block_number),
                Some(pool.token_address.to_string()),
                Some(pool.address.to_string()),
                format!("pool:{}:{block_number}", pool.address),
            ),
            MarketEvent::BlockCompleted { block_number, .. } => (
                Some(*block_number),
                self.market
                    .as_ref()
                    .map(|market| market.token_address.to_string()),
                self.market
                    .as_ref()
                    .and_then(|market| market.pool_address.as_ref().map(ToString::to_string)),
                format!("block_completed:{block_number}"),
            ),
        };
        self.store
            .record_strategy_decision(&strategy_decision_record(
                strategy_name,
                event_source,
                event_key,
                block_number,
                token_address,
                pool_address,
                decision,
            ))
            .await
    }

    async fn record_position_monitor_decision(
        &self,
        strategy_name: &str,
        block_number: u64,
        index: usize,
        decision: &StrategyDecision,
    ) -> Result<()> {
        let intent = decision.order_intent();
        self.store
            .record_strategy_decision(&strategy_decision_record(
                strategy_name,
                "position_monitor",
                format!("position_monitor:{block_number}:{index}"),
                Some(block_number),
                intent.map(|intent| intent.token_address.to_string()),
                intent.map(|intent| intent.pool_address.to_string()),
                decision,
            ))
            .await
    }

    async fn record_risk_strategy_decision(
        &self,
        strategy_name: &str,
        event: &RiskEvent,
        decision: &StrategyDecision,
    ) -> Result<()> {
        let event_source = event.source.as_deref().unwrap_or("risk");
        self.store
            .record_strategy_decision(&strategy_decision_record(
                strategy_name,
                event_source,
                format!(
                    "risk:{}:{}:{}",
                    risk_kind_key(&event.kind),
                    event.token_address,
                    event.observed_block.unwrap_or_default()
                ),
                event.observed_block,
                Some(event.token_address.to_string()),
                event.pool_address.as_ref().map(ToString::to_string),
                decision,
            ))
            .await
    }

    async fn execute_if_allowed(&mut self, intent: OrderIntent) -> Result<Vec<ExecutionReport>> {
        match self.risk_policy.evaluate_order(&intent, &self.active_risks) {
            RiskDecision::Allow | RiskDecision::ReduceSize { .. } => {
                self.execute_intent(intent).await
            }
            RiskDecision::ForceExit { intent, .. } => self.execute_intent(*intent).await,
            RiskDecision::Reject { .. } | RiskDecision::CancelOpenOrders { .. } => {
                self.store.record_order_intent(&intent).await?;
                Ok(Vec::new())
            }
        }
    }

    async fn execute_intent(&mut self, mut intent: OrderIntent) -> Result<Vec<ExecutionReport>> {
        let mut position = self.position_for_intent(&intent);
        intent.trade_id = Some(position.trade_id.clone());
        self.store.record_order_intent(&intent).await?;
        position.mark_intent_created(intent.side)?;
        let report = self.execution.execute(intent.clone()).await?;
        position.mark_order_submitted(report.order_id.clone(), intent.side)?;
        self.store.upsert_position(&position).await?;
        let submission_block = self
            .current_event_block
            .or_else(|| self.market.as_ref().map(|m| m.block_number));
        let mut reports = Vec::new();
        if report.status != ExecutionStatus::Submitted {
            let submitted_report =
                submitted_report_for(&report, submission_block.or(report.block_number));
            self.store
                .record_order_execution_report(&position.id, intent.side, &submitted_report)
                .await?;
            reports.push(submitted_report);
        }

        self.store.upsert_position(&position).await?;
        let position_id = position.id.clone();
        self.portfolio
            .positions
            .insert(position.id.clone(), position);

        if should_defer_report(submission_block, &report) {
            self.pending_execution_reports.push(PendingExecutionReport {
                position_id,
                side: intent.side,
                report,
            });
        } else {
            self.apply_final_execution_report(&position_id, intent.side, &report)
                .await?;
            reports.push(report);
        }

        Ok(reports)
    }

    async fn apply_due_pending_execution_reports(
        &mut self,
        block_number: u64,
    ) -> Result<Vec<ExecutionReport>> {
        let mut due = Vec::new();
        let mut remaining = Vec::new();
        for pending in self.pending_execution_reports.drain(..) {
            let is_due = pending
                .report
                .block_number
                .map(|execution_block| execution_block <= block_number)
                .unwrap_or(true);
            if is_due {
                due.push(pending);
            } else {
                remaining.push(pending);
            }
        }
        self.pending_execution_reports = remaining;
        self.apply_pending_execution_reports(due).await
    }

    async fn apply_pending_execution_reports(
        &mut self,
        mut pending_reports: Vec<PendingExecutionReport>,
    ) -> Result<Vec<ExecutionReport>> {
        pending_reports.sort_by_key(|pending| pending.report.block_number.unwrap_or_default());
        let mut reports = Vec::with_capacity(pending_reports.len());
        for pending in pending_reports {
            self.apply_final_execution_report(&pending.position_id, pending.side, &pending.report)
                .await?;
            reports.push(pending.report);
        }
        Ok(reports)
    }

    async fn apply_final_execution_report(
        &mut self,
        position_id: &PositionId,
        side: OrderSide,
        report: &ExecutionReport,
    ) -> Result<()> {
        let mut position = self
            .portfolio
            .positions
            .get(position_id)
            .cloned()
            .ok_or_else(|| {
                AlphaCoreError::InvalidPositionTransition(format!(
                    "execution report {} has no matching position {}",
                    report.order_id.0, position_id.0
                ))
            })?;
        let report_status = report.status.clone();
        let fill_price = fill_price_for_report(side, report);
        position.apply_execution_report_with_price(report, fill_price)?;

        self.store.upsert_position(&position).await?;
        self.store
            .record_order_execution_report(&position.id, side, report)
            .await?;

        if matches!(
            report_status,
            ExecutionStatus::Failed | ExecutionStatus::Cancelled
        ) {
            if position.drained {
                let block_number = report
                    .block_number
                    .or_else(|| self.market.as_ref().map(|m| m.block_number))
                    .unwrap_or_default();
                let pool = self
                    .pool_snapshots
                    .get(&position.key.pool_address)
                    .filter(|pool| {
                        pool.token_address == position.key.token_address
                            && pool.latest_block == block_number
                    });
                let snapshot = zero_value_snapshot(&position, block_number, pool);
                self.store.append_position_snapshot(&snapshot).await?;
            }
        } else if position.is_closed() {
            let block_number = report
                .block_number
                .or(self.current_event_block)
                .or_else(|| self.market.as_ref().map(|m| m.block_number))
                .unwrap_or_default();
            let pool = valuation_safe_pool(
                self.pool_snapshots.get(&position.key.pool_address),
                block_number,
            );
            let snapshot = PositionSnapshot {
                position_id: position.id.clone(),
                trade_id: position.trade_id.clone(),
                state: position.state.clone(),
                block_number,
                observed_block_number: pool.map(|pool| pool.latest_block).or(Some(block_number)),
                valuation_block_number: Some(block_number),
                current_value_eth: DecimalAmount::ZERO,
                realized_profit_eth: position.realized_pnl(),
                unrealized_profit_eth: DecimalAmount::ZERO,
                roi: if let Some(cost) = position.entry_cost_basis {
                    if !cost.is_zero() {
                        position.realized_pnl() / cost
                    } else {
                        DecimalAmount::ZERO
                    }
                } else {
                    DecimalAmount::ZERO
                },
                pool_price_to_initial_price_ratio: None,
                pool_initial_price_denom_per_token: None,
                pool_price_denom_per_token: None,
                pool_liquidity_denom: None,
                pool_token_reserve: None,
                pool_denom_symbol: None,
            };
            let snapshot = snapshot_with_pool_metrics(snapshot, pool);
            self.store.append_position_snapshot(&snapshot).await?;
        } else if side == OrderSide::Buy && report_status == ExecutionStatus::Confirmed {
            self.snapshot_confirmed_buy_position(&position).await?;
        }

        self.portfolio
            .positions
            .insert(position.id.clone(), position);
        Ok(())
    }

    async fn snapshot_confirmed_buy_position(&mut self, position: &Position) -> Result<()> {
        if position.state != PositionState::BuyConfirmed {
            return Ok(());
        }

        let valuation_block = position
            .entry_block
            .or(self.current_event_block)
            .unwrap_or_default();
        let Some(pool) = valuation_safe_pool(
            self.pool_snapshots.get(&position.key.pool_address),
            valuation_block,
        )
        .cloned() else {
            return Ok(());
        };

        if let Some(value) = self
            .execution
            .simulate_position_value(position, &pool)
            .await?
        {
            let snapshot = simulated_value_snapshot(position, value, Some(&pool));
            self.store.append_position_snapshot(&snapshot).await?;
        }

        Ok(())
    }

    fn position_for_intent(&self, intent: &OrderIntent) -> Position {
        let key = PositionKey {
            portfolio_id: intent.portfolio_id.clone(),
            wallet_id: intent.wallet_id.clone(),
            strategy_name: intent.strategy_name.clone(),
            token_address: intent.token_address,
            pool_address: intent.pool_address.clone(),
            protocol: intent.protocol.clone(),
        };
        if let Some(position) = self
            .portfolio
            .positions
            .values()
            .find(|position| {
                intent
                    .trade_id
                    .as_ref()
                    .map(|trade_id| position.trade_id == *trade_id)
                    .unwrap_or(false)
            })
            .cloned()
        {
            return position;
        }
        if intent.side == OrderSide::Sell {
            if let Some(position) = self
                .portfolio
                .positions
                .values()
                .find(|position| position.key == key && position.can_submit_exit())
                .cloned()
            {
                return position;
            }
        }
        let trade_id = intent.trade_id.clone().unwrap_or_else(new_trade_id);
        let id = PositionId(trade_id.0.clone());
        Position::with_trade_id(id, trade_id, key)
    }
}

#[cfg(test)]
mod tests;
