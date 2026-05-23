use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use eth_alpha_core::{
    error::Result,
    execution::ExecutionReport,
    order::OrderIntent,
    position::{Position, PositionSnapshot},
    risk::RiskEvent,
    store::{StrategyDecisionRecord, TradingStore},
};

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
        let mut snapshots = self.snapshots.lock().expect("store lock");
        if let Some(existing) = snapshots.iter_mut().find(|existing| {
            existing.position_id == snapshot.position_id
                && existing.trade_id == snapshot.trade_id
                && existing.state == snapshot.state
                && existing.block_number == snapshot.block_number
                && existing.valuation_block_number == snapshot.valuation_block_number
        }) {
            *existing = snapshot.clone();
        } else {
            snapshots.push(snapshot.clone());
        }
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
