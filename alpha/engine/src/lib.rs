//! High-level alpha runtime skeleton.
//!
//! The engine sits above live feed/state and mempool risk. It consumes typed
//! market, risk, and execution events, runs strategies, applies risk policy, and
//! routes approved intents to an execution adapter. The no-capital runtime uses
//! chain-state EVM simulation; real submission remains isolated behind a future
//! `tx_executor` adapter.

pub mod execution;
pub mod live_trader;
pub mod wire;

mod decision;
mod ids;
mod policy;
mod runtime;
mod store;
mod valuation;

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use eth_alpha_core::{
    amount::Amount,
    error::Result,
    execution::ExecutionReport,
    ids::{PositionId, TokenPoolId},
    market::{MarketEvent, MarketSnapshotRef, PoolSnapshot},
    order::{OrderIntent, OrderSide},
    portfolio::PortfolioState,
    risk::{RiskEvent, RiskPolicy},
    store::TradingStore,
    strategy::Strategy,
};

// Re-export simulation adapters at crate root for convenience. Real tx
// execution stays inside `execution::real` and `live_trader::real_execution` so
// historical and no-capital backtests cannot import it accidentally.
pub use execution::{ChainSimExecutionAdapter, LiveChainSimExecutionAdapter};
pub use policy::{AllowAllRiskPolicy, BlockCriticalRiskPolicy};
pub use store::MemoryTradingStore;

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
pub(crate) struct PendingExecutionReport {
    pub(crate) position_id: PositionId,
    pub(crate) side: OrderSide,
    pub(crate) report: ExecutionReport,
}

#[async_trait]
pub trait EngineExecutionAdapter: Send + Sync {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport>;

    async fn simulate_position_value(
        &self,
        _position: &eth_alpha_core::position::Position,
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
        position: &eth_alpha_core::position::Position,
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
    written_snapshot_keys: HashSet<String>,
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
            written_snapshot_keys: HashSet::new(),
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
}

#[cfg(test)]
mod gate3_validation;
#[cfg(test)]
mod tests;
