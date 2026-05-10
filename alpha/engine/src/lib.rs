//! High-level alpha runtime skeleton.
//!
//! The engine sits above live feed/state and mempool risk. It consumes typed
//! market, risk, and execution events, runs strategies, applies risk policy, and
//! routes approved intents to an execution adapter. The first runtime mode is
//! paper execution, so this crate deliberately does not talk to `tx_executor`.

pub mod execution;
pub mod wire;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use eth_alpha_core::{
    amount::DecimalAmount,
    error::Result,
    execution::ExecutionReport,
    market::{MarketEvent, MarketSnapshotRef},
    order::{OrderIntent, OrderSide},
    portfolio::PortfolioState,
    position::{Position, PositionKey, PositionSnapshot},
    risk::{RiskDecision, RiskEvent, RiskKind, RiskPolicy, RiskSeverity},
    store::TradingStore,
    strategy::{Strategy, StrategyContext, StrategyDecision},
};

// Re-export adapters at crate root for convenience.
pub use execution::{ExecutionAdapterKind, ModeledExecutionAdapter, ModeledExecutionConfig, PaperExecutionAdapter};

#[derive(Clone, Debug, PartialEq)]
pub enum EngineEvent {
    Market(MarketEvent),
    Risk(RiskEvent),
    Execution(ExecutionReport),
}

#[async_trait]
pub trait EngineExecutionAdapter: Send + Sync {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport>;
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
        match event {
            EngineEvent::Market(event) => {
                self.apply_market_event(&event);
                self.run_market_strategies(&event).await
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
        let Some(market) = self.market.clone() else {
            return Ok(Vec::new());
        };
        let portfolio = self.portfolio.clone();
        let active_risks = self.active_risks.clone();
        let ctx = StrategyContext {
            market: &market,
            portfolio: &portfolio,
            active_risks: &active_risks,
        };

        let mut decisions = Vec::with_capacity(self.strategies.len());
        for strategy in &mut self.strategies {
            decisions.push(strategy.on_market_event(&ctx, event)?);
        }
        let reports = self.apply_decisions(decisions).await?;
        self.snapshot_open_positions_for_pool(event).await?;
        Ok(reports)
    }

    async fn snapshot_open_positions_for_pool(&mut self, event: &MarketEvent) -> Result<()> {
        let MarketEvent::PoolUpdated { pool, block_number } = event else {
            return Ok(());
        };
        for position in self.portfolio.positions.values_mut() {
            if position.key.pool_address != pool.address {
                continue;
            }
            if !position.is_open() {
                continue;
            }
            let (current_value, unrealized) = position.unrealized_pnl(pool);
            let snapshot = PositionSnapshot {
                position_id: position.id.clone(),
                state: position.state.clone(),
                block_number: *block_number,
                current_value_eth: current_value,
                realized_profit_eth: position.realized_pnl(),
                unrealized_profit_eth: unrealized,
                roi: if let Some(cost) = position.entry_cost_basis {
                    if !cost.is_zero() {
                        ((current_value + position.realized_pnl()) / cost)
                            - DecimalAmount::from(1)
                    } else {
                        DecimalAmount::ZERO
                    }
                } else {
                    DecimalAmount::ZERO
                },
            };
            self.store.append_position_snapshot(&snapshot).await?;
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
        for strategy in &mut self.strategies {
            decisions.push(strategy.on_risk_event(&ctx, event)?);
        }
        let reports = self.apply_decisions(decisions).await?;

        // Worst-case baseline: mark open positions as drained on
        // liquidity removal or scam confirmation, even if strategy does not exit.
        if matches!(event.kind, RiskKind::LiquidityRemoval | RiskKind::ScamConfirmed) {
            if let Some(ref pool_address) = event.pool_address {
                for position in self.portfolio.positions.values_mut() {
                    if position.key.pool_address == *pool_address
                        && position.is_open()
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
                            state: position.state.clone(),
                            block_number: event.observed_block.unwrap_or(u64::MAX - 1),
                            current_value_eth: DecimalAmount::ZERO,
                            realized_profit_eth: position.realized_pnl(),
                            unrealized_profit_eth: -position.entry_cost_basis.unwrap_or_default(),
                            roi: DecimalAmount::from(-1),
                        };
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
                StrategyDecision::Hold | StrategyDecision::CancelOrders { .. } => {}
                StrategyDecision::SubmitOrder(intent) => {
                    reports.extend(self.execute_if_allowed(intent).await?);
                }
            }
        }
        Ok(reports)
    }

    async fn execute_if_allowed(&mut self, intent: OrderIntent) -> Result<Vec<ExecutionReport>> {
        self.store.record_order_intent(&intent).await?;
        let fill_price = self.market.as_ref().and_then(|m| {
            m.pool.as_ref().and_then(|p| p.price_denom_per_token)
        });
        match self.risk_policy.evaluate_order(&intent, &self.active_risks) {
            RiskDecision::Allow | RiskDecision::ReduceSize { .. } => {
                let report = self.execute_intent(intent, fill_price).await?;
                Ok(vec![report])
            }
            RiskDecision::ForceExit { intent, .. } => {
                let report = self.execute_intent(*intent, fill_price).await?;
                Ok(vec![report])
            }
            RiskDecision::Reject { .. } | RiskDecision::CancelOpenOrders { .. } => Ok(Vec::new()),
        }
    }

    async fn execute_intent(
        &mut self,
        intent: OrderIntent,
        snapshot_price: Option<DecimalAmount>,
    ) -> Result<ExecutionReport> {
        let mut position = self.position_for_intent(&intent);
        position.mark_intent_created(intent.side)?;
        let report = self.execution.execute(intent.clone()).await?;
        position.mark_order_submitted(report.order_id.clone(), intent.side)?;

        // When the execution adapter provides both cost basis and token amount,
        // compute the actual fill price from the report rather than using the
        // pool snapshot price. This gives chain-parity pricing that accounts
        // for real slippage, taxes, and price impact.
        let fill_price = match intent.side {
            OrderSide::Buy => {
                if let (Some(cost), Some(tokens)) = (&report.filled_amount, &report.token_amount) {
                    let cost_dec = cost.to_decimal();
                    let token_dec = tokens.to_decimal();
                    if !token_dec.is_zero() {
                        Some(cost_dec / token_dec)
                    } else {
                        snapshot_price
                    }
                } else {
                    snapshot_price
                }
            }
            OrderSide::Sell => snapshot_price,
        };
        position.apply_execution_report_with_price(&report, fill_price)?;

        self.store.upsert_position(&position).await?;
        self.store.record_execution_report(&report).await?;
        if position.is_closed() {
            let snapshot = PositionSnapshot {
                position_id: position.id.clone(),
                state: position.state.clone(),
                block_number: self.market.as_ref().map(|m| m.block_number).unwrap_or_default(),
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
            };
            self.store.append_position_snapshot(&snapshot).await?;
        }
        self.portfolio
            .positions
            .insert(position.id.clone(), position);
        Ok(report)
    }

    fn position_for_intent(&self, intent: &OrderIntent) -> Position {
        let key = PositionKey {
            portfolio_id: intent.portfolio_id.clone(),
            wallet_id: intent.wallet_id.clone(),
            strategy_name: intent.strategy_name.clone(),
            token_address: intent.token_address,
            pool_address: intent.pool_address.clone(),
        };
        let id = position_id_for_key(&key);
        self.portfolio
            .positions
            .get(&id)
            .cloned()
            .unwrap_or_else(|| Position::new(id, key))
    }
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

fn position_id_for_key(key: &PositionKey) -> eth_alpha_core::ids::PositionId {
    eth_alpha_core::ids::PositionId(format!(
        "{}:{}:{}:{}:{}",
        key.portfolio_id.0,
        key.wallet_id.0,
        key.strategy_name.0,
        key.token_address,
        key.pool_address
    ))
}

#[derive(Clone, Default)]
pub struct MemoryTradingStore {
    positions: Arc<Mutex<Vec<Position>>>,
    snapshots: Arc<Mutex<Vec<PositionSnapshot>>>,
    order_intents: Arc<Mutex<Vec<OrderIntent>>>,
    execution_reports: Arc<Mutex<Vec<ExecutionReport>>>,
    risk_events: Arc<Mutex<Vec<RiskEvent>>>,
}

impl MemoryTradingStore {
    pub fn positions(&self) -> Vec<Position> {
        self.positions.lock().expect("store lock").clone()
    }

    pub fn order_intents(&self) -> Vec<OrderIntent> {
        self.order_intents.lock().expect("store lock").clone()
    }

    pub fn execution_reports(&self) -> Vec<ExecutionReport> {
        self.execution_reports.lock().expect("store lock").clone()
    }

    pub fn risk_events(&self) -> Vec<RiskEvent> {
        self.risk_events.lock().expect("store lock").clone()
    }
}

#[async_trait]
impl TradingStore for MemoryTradingStore {
    async fn upsert_position(&self, position: &Position) -> Result<()> {
        self.positions
            .lock()
            .expect("store lock")
            .push(position.clone());
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
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, U256};
    use eth_alpha_core::{
        amount::Amount,
        execution::ExecutionStatus,
        ids::{PortfolioId, StrategyName, TokenPoolId, WalletId},
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
                portfolio_id: PortfolioId("paper".to_string()),
                wallet_id: WalletId("paper-wallet".to_string()),
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

    #[tokio::test]
    async fn paper_engine_executes_approved_strategy_order() {
        let store = MemoryTradingStore::default();
        let mut engine = AlphaEngine::new(
            AllowAllRiskPolicy,
            store.clone(),
            PaperExecutionAdapter::new(),
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
                    token_decimals: None,
                    latest_block: 1,
                    can_buy: true,
                    can_sell: true,
                    is_scam: false,
                },
            }))
            .await
            .unwrap();

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].status, ExecutionStatus::Confirmed);
        assert_eq!(store.order_intents().len(), 1);
        assert_eq!(store.execution_reports().len(), 1);
        assert_eq!(store.positions().len(), 1);
        assert_eq!(engine.portfolio().active_position_count(), 1);
    }

    #[tokio::test]
    async fn default_risk_event_strategy_hook_holds() {
        let store = MemoryTradingStore::default();
        let mut engine = AlphaEngine::new(
            AllowAllRiskPolicy,
            store.clone(),
            PaperExecutionAdapter::new(),
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
    }

    #[tokio::test]
    async fn critical_risk_policy_rejects_matching_order() {
        let store = MemoryTradingStore::default();
        let mut engine = AlphaEngine::new(
            BlockCriticalRiskPolicy,
            store.clone(),
            PaperExecutionAdapter::new(),
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
                    token_decimals: None,
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
    }
}
