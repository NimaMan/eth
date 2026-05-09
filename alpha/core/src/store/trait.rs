use async_trait::async_trait;

use crate::{
    error::Result,
    execution::ExecutionReport,
    order::OrderIntent,
    position::{Position, PositionSnapshot},
    risk::RiskEvent,
};

#[async_trait]
pub trait TradingStore: Send + Sync {
    async fn upsert_position(&self, position: &Position) -> Result<()>;

    async fn append_position_snapshot(&self, snapshot: &PositionSnapshot) -> Result<()>;

    async fn record_order_intent(&self, intent: &OrderIntent) -> Result<()>;

    async fn record_execution_report(&self, report: &ExecutionReport) -> Result<()>;

    async fn record_risk_event(&self, _event: &RiskEvent) -> Result<()> {
        Ok(())
    }
}
