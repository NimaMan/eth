//! High-level alpha runtime skeleton.
//!
//! The engine sits above live feed/state and mempool risk. It consumes typed
//! market, risk, and execution events, runs strategies, applies risk policy, and
//! routes approved intents to an execution adapter. The no-capital runtime uses
//! chain-state EVM simulation; real submission remains isolated behind a future
//! `tx_executor` adapter.

pub mod execution;
pub mod wire;

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    error::{AlphaCoreError, Result},
    execution::{ExecutionReport, ExecutionStatus},
    ids::{PositionId, TokenPoolId, TradeId},
    market::{MarketEvent, MarketSnapshotRef, PoolSnapshot},
    order::{OrderIntent, OrderSide},
    portfolio::PortfolioState,
    position::{Position, PositionKey, PositionSnapshot, PositionState},
    risk::{RiskDecision, RiskEvent, RiskKind, RiskPolicy, RiskSeverity},
    store::{StrategyDecisionRecord, TradingStore},
    strategy::{Strategy, StrategyContext, StrategyDecision},
};
use serde_json::json;

// Re-export chain-simulation adapters at crate root for convenience.
pub use execution::{ChainSimExecutionAdapter, LiveChainSimExecutionAdapter};

static TRADE_COUNTER: AtomicU64 = AtomicU64::new(1);

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
                self.active_risks.push(event.clone());
                self.store.record_risk_event(&event).await?;
                self.run_risk_strategies(&event).await
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

        let mut decisions = Vec::with_capacity(self.strategies.len());
        let mut market_decisions_to_record = Vec::new();
        let mut monitor_decisions_to_record = Vec::new();
        for strategy in &mut self.strategies {
            let strategy_name = strategy.name();
            let decision = strategy.on_market_event(&ctx, event)?;
            if !matches!(event, MarketEvent::BlockCompleted { .. }) {
                market_decisions_to_record.push((strategy_name.0.clone(), decision.clone()));
            }
            decisions.push(decision);
            if let MarketEvent::BlockCompleted { block_number, .. } = event {
                let monitor_decisions = strategy.on_position_monitor(&ctx, *block_number)?;
                for (index, decision) in monitor_decisions.iter().enumerate() {
                    monitor_decisions_to_record.push((
                        strategy_name.0.clone(),
                        *block_number,
                        index,
                        decision.clone(),
                    ));
                }
                decisions.extend(monitor_decisions);
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
        let reports = self.apply_decisions(decisions).await?;
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
            let snapshot = if position.drained
                || matches!(
                    position.state,
                    PositionState::SellFailed | PositionState::SellCancelled
                ) {
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
        let reports = self.apply_decisions(decisions).await?;

        // Worst-case baseline: mark open positions as drained on
        // liquidity removal or scam confirmation, even if strategy does not exit.
        if matches!(
            event.kind,
            RiskKind::LiquidityRemoval | RiskKind::ScamConfirmed
        ) {
            if let Some(ref pool_address) = event.pool_address {
                let pool_snapshot = self.pool_snapshots.get(pool_address).cloned();
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
                        let snapshot = PositionSnapshot {
                            position_id: position.id.clone(),
                            trade_id: position.trade_id.clone(),
                            state: position.state.clone(),
                            block_number: event.observed_block.unwrap_or(u64::MAX - 1),
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
                        let snapshot = snapshot_with_pool_metrics(snapshot, pool_snapshot.as_ref());
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
    ) -> Result<Vec<ExecutionReport>> {
        let mut reports = Vec::new();
        for decision in decisions {
            match decision {
                StrategyDecision::Hold
                | StrategyDecision::HoldWithReason { .. }
                | StrategyDecision::CancelOrders { .. } => {}
                StrategyDecision::SubmitOrder(intent)
                | StrategyDecision::SubmitOrderWithReason { intent, .. } => {
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
        self.store
            .record_strategy_decision(&strategy_decision_record(
                strategy_name,
                "risk",
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
            let pool = self.pool_snapshots.get(&position.key.pool_address);
            let snapshot = zero_value_snapshot(
                &position,
                report
                    .block_number
                    .or_else(|| self.market.as_ref().map(|m| m.block_number))
                    .unwrap_or_default(),
                pool,
            );
            self.store.append_position_snapshot(&snapshot).await?;
        } else if position.is_closed() {
            let pool = self.pool_snapshots.get(&position.key.pool_address);
            let snapshot = PositionSnapshot {
                position_id: position.id.clone(),
                trade_id: position.trade_id.clone(),
                state: position.state.clone(),
                block_number: report
                    .block_number
                    .or(self.current_event_block)
                    .or_else(|| self.market.as_ref().map(|m| m.block_number))
                    .unwrap_or_default(),
                observed_block_number: self.current_event_block.or_else(|| {
                    self.market
                        .as_ref()
                        .and_then(|market| market.pool.as_ref().map(|pool| pool.latest_block))
                }),
                valuation_block_number: report.block_number,
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

        let Some(pool) = self.pool_snapshots.get(&position.key.pool_address).cloned() else {
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

fn submitted_report_for(report: &ExecutionReport, block_number: Option<u64>) -> ExecutionReport {
    ExecutionReport {
        order_id: report.order_id.clone(),
        status: ExecutionStatus::Submitted,
        tx_hash: report.tx_hash,
        block_number,
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        error: None,
    }
}

fn should_defer_report(submission_block: Option<u64>, report: &ExecutionReport) -> bool {
    if matches!(
        report.status,
        ExecutionStatus::Submitted | ExecutionStatus::Pending
    ) {
        return false;
    }
    match (submission_block, report.block_number) {
        (Some(submitted_at), Some(executed_at)) => executed_at > submitted_at,
        _ => false,
    }
}

fn fill_price_for_report(side: OrderSide, report: &ExecutionReport) -> Option<DecimalAmount> {
    if side != OrderSide::Buy {
        return None;
    }
    let (Some(cost), Some(tokens)) = (&report.filled_amount, &report.token_amount) else {
        return None;
    };
    let token_dec = tokens.to_decimal();
    if token_dec.is_zero() {
        return None;
    }
    Some(cost.to_decimal() / token_dec)
}

fn engine_event_block(event: &EngineEvent) -> Option<u64> {
    match event {
        EngineEvent::Market(
            MarketEvent::TokenUpdated { block_number, .. }
            | MarketEvent::PoolUpdated { block_number, .. }
            | MarketEvent::BlockCompleted { block_number, .. },
        ) => Some(*block_number),
        EngineEvent::Risk(risk) => risk.observed_block,
        EngineEvent::Execution(report) => report.block_number,
    }
}

fn market_event_block(event: &MarketEvent) -> u64 {
    match event {
        MarketEvent::TokenUpdated { block_number, .. }
        | MarketEvent::PoolUpdated { block_number, .. }
        | MarketEvent::BlockCompleted { block_number, .. } => *block_number,
    }
}

fn should_snapshot_position_for_pool(position: &Position) -> bool {
    if position.drained {
        return position.has_exposure();
    }
    matches!(
        position.state,
        PositionState::BuyConfirmed | PositionState::SellFailed | PositionState::SellCancelled
    )
}

#[derive(Clone, Debug, Default)]
pub struct AllowAllRiskPolicy;

impl RiskPolicy for AllowAllRiskPolicy {
    fn evaluate_order(&self, _intent: &OrderIntent, _active_risks: &[RiskEvent]) -> RiskDecision {
        RiskDecision::Allow
    }
}

#[derive(Clone, Debug, Default)]
pub struct BlockCriticalRiskPolicy;

impl RiskPolicy for BlockCriticalRiskPolicy {
    fn evaluate_order(&self, intent: &OrderIntent, active_risks: &[RiskEvent]) -> RiskDecision {
        if intent.side == OrderSide::Sell {
            return RiskDecision::Allow;
        }
        if let Some(risk) = active_risks.iter().rev().find(|risk| {
            risk.severity == RiskSeverity::Critical
                && risk.kind != RiskKind::TradingEnabled
                && risk.token_address == intent.token_address
                && risk
                    .pool_address
                    .as_ref()
                    .map(|pool| pool == &intent.pool_address)
                    .unwrap_or(true)
        }) {
            return RiskDecision::Reject {
                reason: format!("critical active risk for order: {}", risk.message),
            };
        }
        RiskDecision::Allow
    }
}

fn new_trade_id() -> TradeId {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default();
    let sequence = TRADE_COUNTER.fetch_add(1, Ordering::Relaxed);
    TradeId(format!(
        "trd_{}_{}_{}",
        base36(millis),
        base36(std::process::id() as u64),
        base36(sequence)
    ))
}

#[cfg(test)]
fn position_id_for_key(key: &PositionKey) -> eth_alpha_core::ids::PositionId {
    let seed = TRADE_COUNTER.fetch_add(1, Ordering::Relaxed);
    eth_alpha_core::ids::PositionId(format!(
        "legacy_{}_{}_{}_{}",
        sanitize_id_part(&key.strategy_name.0),
        key.token_address,
        key.pool_address,
        base36(seed)
    ))
}

fn base36(mut value: u64) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while value > 0 {
        let digit = (value % 36) as u8;
        out.push(match digit {
            0..=9 => b'0' + digit,
            _ => b'a' + (digit - 10),
        });
        value /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
fn sanitize_id_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn strategy_decision_record(
    strategy_name: &str,
    event_source: &str,
    event_key: String,
    block_number: Option<u64>,
    token_address: Option<String>,
    pool_address: Option<String>,
    decision: &StrategyDecision,
) -> StrategyDecisionRecord {
    let order = decision.order_intent();
    StrategyDecisionRecord {
        strategy_name: strategy_name.to_string(),
        event_source: event_source.to_string(),
        event_key,
        block_number,
        token_address: token_address
            .or_else(|| order.map(|intent| intent.token_address.to_string())),
        pool_address: pool_address.or_else(|| order.map(|intent| intent.pool_address.to_string())),
        action: strategy_decision_action(decision).to_string(),
        reason: decision.reason().map(ToOwned::to_owned),
        order_side: order.map(|intent| intent.side),
        payload: json!({
            "decision": decision,
        }),
    }
}

fn strategy_decision_action(decision: &StrategyDecision) -> &'static str {
    match decision {
        StrategyDecision::Hold | StrategyDecision::HoldWithReason { .. } => "hold",
        StrategyDecision::SubmitOrder(intent)
        | StrategyDecision::SubmitOrderWithReason { intent, .. } => match intent.side {
            OrderSide::Buy => "submit_buy",
            OrderSide::Sell => "submit_sell",
        },
        StrategyDecision::CancelOrders { .. } => "cancel_orders",
    }
}

fn risk_kind_key(kind: &RiskKind) -> String {
    match kind {
        RiskKind::LiquidityRemoval => "liquidity_removal".to_string(),
        RiskKind::TaxChange => "tax_change".to_string(),
        RiskKind::Honeypot => "honeypot".to_string(),
        RiskKind::TradingDisabled => "trading_disabled".to_string(),
        RiskKind::TradingEnabled => "trading_enabled".to_string(),
        RiskKind::LpApproval => "lp_approval".to_string(),
        RiskKind::ScamConfirmed => "scam_confirmed".to_string(),
        RiskKind::Custom(value) => value.clone(),
    }
}

fn zero_value_snapshot(
    position: &Position,
    block_number: u64,
    pool: Option<&PoolSnapshot>,
) -> PositionSnapshot {
    let cost = position.entry_cost_basis.unwrap_or_default();
    let realized = position.realized_pnl();
    let unrealized = if position.is_closed() {
        DecimalAmount::ZERO
    } else {
        -cost
    };
    let roi = if cost.is_zero() {
        DecimalAmount::ZERO
    } else if position.is_closed() {
        realized / cost
    } else {
        DecimalAmount::from(-1)
    };
    let snapshot = PositionSnapshot {
        position_id: position.id.clone(),
        trade_id: position.trade_id.clone(),
        state: position.state.clone(),
        block_number,
        observed_block_number: pool.map(|pool| pool.latest_block).or(Some(block_number)),
        valuation_block_number: Some(block_number),
        current_value_eth: DecimalAmount::ZERO,
        realized_profit_eth: realized,
        unrealized_profit_eth: unrealized,
        roi,
        pool_price_to_initial_price_ratio: None,
        pool_initial_price_denom_per_token: None,
        pool_price_denom_per_token: None,
        pool_liquidity_denom: None,
        pool_token_reserve: None,
        pool_denom_symbol: None,
    };
    snapshot_with_pool_metrics(snapshot, pool)
}

fn simulated_value_snapshot(
    position: &Position,
    simulation: PositionValueSimulation,
    pool: Option<&PoolSnapshot>,
) -> PositionSnapshot {
    let current_value = simulation.current_value.to_decimal();
    let cost = position.entry_cost_basis.unwrap_or_default();
    let realized = position.realized_pnl();
    let snapshot = PositionSnapshot {
        position_id: position.id.clone(),
        trade_id: position.trade_id.clone(),
        state: position.state.clone(),
        block_number: simulation.block_number,
        observed_block_number: pool.map(|pool| pool.latest_block),
        valuation_block_number: Some(simulation.block_number),
        current_value_eth: current_value,
        realized_profit_eth: realized,
        unrealized_profit_eth: if cost.is_zero() {
            DecimalAmount::ZERO
        } else {
            current_value - cost
        },
        roi: if cost.is_zero() {
            DecimalAmount::ZERO
        } else {
            ((current_value + realized) / cost) - DecimalAmount::from(1)
        },
        pool_price_to_initial_price_ratio: None,
        pool_initial_price_denom_per_token: None,
        pool_price_denom_per_token: None,
        pool_liquidity_denom: None,
        pool_token_reserve: None,
        pool_denom_symbol: None,
    };
    snapshot_with_pool_metrics(snapshot, pool)
}

fn snapshot_with_pool_metrics(
    mut snapshot: PositionSnapshot,
    pool: Option<&PoolSnapshot>,
) -> PositionSnapshot {
    let Some(pool) = pool else {
        return snapshot;
    };

    snapshot.pool_price_to_initial_price_ratio = pool.price_ratio_to_initial;
    snapshot.pool_initial_price_denom_per_token = pool.initial_price_denom_per_token;
    snapshot.pool_price_denom_per_token = pool.price_denom_per_token;
    snapshot.pool_liquidity_denom = Some(pool.denom_reserve);
    snapshot.pool_token_reserve = Some(pool.token_reserve);
    snapshot.pool_denom_symbol = pool.denom_symbol.clone();

    snapshot
}

#[derive(Clone, Default)]
pub struct MemoryTradingStore {
    positions: Arc<Mutex<Vec<Position>>>,
    snapshots: Arc<Mutex<Vec<PositionSnapshot>>>,
    order_intents: Arc<Mutex<Vec<OrderIntent>>>,
    execution_reports: Arc<Mutex<Vec<ExecutionReport>>>,
    risk_events: Arc<Mutex<Vec<RiskEvent>>>,
    strategy_decisions: Arc<Mutex<Vec<StrategyDecisionRecord>>>,
}

impl MemoryTradingStore {
    pub fn positions(&self) -> Vec<Position> {
        self.positions.lock().expect("store lock").clone()
    }

    pub fn order_intents(&self) -> Vec<OrderIntent> {
        self.order_intents.lock().expect("store lock").clone()
    }

    pub fn snapshots(&self) -> Vec<PositionSnapshot> {
        self.snapshots.lock().expect("store lock").clone()
    }

    pub fn execution_reports(&self) -> Vec<ExecutionReport> {
        self.execution_reports.lock().expect("store lock").clone()
    }

    pub fn risk_events(&self) -> Vec<RiskEvent> {
        self.risk_events.lock().expect("store lock").clone()
    }

    pub fn strategy_decisions(&self) -> Vec<StrategyDecisionRecord> {
        self.strategy_decisions.lock().expect("store lock").clone()
    }
}

#[async_trait]
impl TradingStore for MemoryTradingStore {
    async fn upsert_position(&self, position: &Position) -> Result<()> {
        let mut positions = self.positions.lock().expect("store lock");
        if let Some(existing) = positions
            .iter_mut()
            .find(|existing| existing.id == position.id)
        {
            *existing = position.clone();
        } else {
            positions.push(position.clone());
        }
        Ok(())
    }

    async fn append_position_snapshot(&self, snapshot: &PositionSnapshot) -> Result<()> {
        self.snapshots
            .lock()
            .expect("store lock")
            .push(snapshot.clone());
        Ok(())
    }

    async fn record_order_intent(&self, intent: &OrderIntent) -> Result<()> {
        self.order_intents
            .lock()
            .expect("store lock")
            .push(intent.clone());
        Ok(())
    }

    async fn record_execution_report(&self, report: &ExecutionReport) -> Result<()> {
        self.execution_reports
            .lock()
            .expect("store lock")
            .push(report.clone());
        Ok(())
    }

    async fn record_risk_event(&self, event: &RiskEvent) -> Result<()> {
        self.risk_events
            .lock()
            .expect("store lock")
            .push(event.clone());
        Ok(())
    }

    async fn record_strategy_decision(&self, record: &StrategyDecisionRecord) -> Result<()> {
        self.strategy_decisions
            .lock()
            .expect("store lock")
            .push(record.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, U256};
    use eth_alpha_core::{
        amount::{Amount, DecimalAmount},
        execution::ExecutionStatus,
        ids::{OrderId, PortfolioId, StrategyName, TokenPoolId, WalletId},
        market::{PoolProtocol, PoolSnapshot},
        order::{OrderIntent, OrderSide},
        risk::{RiskKind, RiskSeverity},
        Result, Strategy,
    };

    use super::*;

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
                amount: Amount {
                    raw: U256::from(1_000_000u64),
                    decimals: 18,
                },
                route: None,
                max_slippage_bps: 500,
                deadline_secs: 30,
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
                position.key.strategy_name == self.name()
                    && position.key.pool_address == pool.address
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
                            amount,
                            route: None,
                            max_slippage_bps: 500,
                            deadline_secs: 30,
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
                    amount: Amount {
                        raw: U256::from(1_000_000u64),
                        decimals: 18,
                    },
                    route: None,
                    max_slippage_bps: 500,
                    deadline_secs: 30,
                },
                "entry.first_pool_update",
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
                            amount: position.entry_token_raw_amount.clone().unwrap_or(Amount {
                                raw: U256::from(1_000_000u64),
                                decimals: 18,
                            }),
                            route: None,
                            max_slippage_bps: 500,
                            deadline_secs: 30,
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
            token_decimals: None,
            fee_tier: None,
            uniswap_v4: None,
            latest_block: block_number,
            can_buy: true,
            can_sell: true,
            is_scam: false,
        }
    }

    fn test_position(state: PositionState) -> Position {
        let token = Address::repeat_byte(0x11);
        let pool_address = Address::repeat_byte(0x22);
        let mut position = Position::new(
            PositionId("test-position".to_string()),
            PositionKey {
                portfolio_id: PortfolioId("chain-sim".to_string()),
                wallet_id: WalletId("chain-sim-wallet".to_string()),
                strategy_name: StrategyName("strategy".to_string()),
                token_address: token,
                pool_address: TokenPoolId::new(token, pool_address.to_string()),
            },
        );
        position.state = state;
        position
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
                token_address: Address::repeat_byte(0x33),
                pool_address: Some(TokenPoolId::new(
                    Address::repeat_byte(0x33),
                    Address::repeat_byte(0x44).to_string(),
                )),
                pending_tx_hash: None,
                observed_block: None,
                message: "lp approval".to_string(),
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
                token_address: token,
                pool_address: Some(pool.clone()),
                pending_tx_hash: None,
                observed_block: Some(1),
                message: "liquidity removal".to_string(),
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
}
