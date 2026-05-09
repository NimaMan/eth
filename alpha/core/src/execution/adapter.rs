use async_trait::async_trait;

use crate::{error::Result, ids::OrderId, order::OrderIntent};

#[async_trait]
pub trait ExecutionAdapter: Send + Sync {
    async fn submit(&self, intent: OrderIntent) -> Result<OrderId>;
}
