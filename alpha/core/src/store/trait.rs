use async_trait::async_trait;

use crate::{
    error::Result,
    execution::ExecutionReport,
    ids::PositionId,
    order::{OrderIntent, OrderSide},
    position::{Position, PositionSnapshot},
    risk::RiskEvent,
};

#[async_trait]
pub trait TradingStore: Send + Sync {
    async fn upsert_position(&self, position: &Position) -> Result<()>;

    async fn append_position_snapshot(&self, snapshot: &PositionSnapshot) -> Result<()>;

    async fn record_order_intent(&self, intent: &OrderIntent) -> Result<()>;

    async fn record_execution_report(&self, report: &ExecutionReport) -> Result<()>;

    async fn record_position_execution_report(
        &self,
        _position_id: &PositionId,
        report: &ExecutionReport,
    ) -> Result<()> {
        self.record_execution_report(report).await
    }

    async fn record_order_execution_report(
        &self,
        position_id: &PositionId,
        _side: OrderSide,
        report: &ExecutionReport,
    ) -> Result<()> {
        self.record_position_execution_report(position_id, report)
            .await
    }

    async fn record_risk_event(&self, _event: &RiskEvent) -> Result<()> {
        Ok(())
    }
}
