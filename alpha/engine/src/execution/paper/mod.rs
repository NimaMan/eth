use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use eth_alpha_core::{
    error::Result,
    execution::{ExecutionReport, ExecutionStatus},
    ids::OrderId,
    order::OrderIntent,
};

use crate::EngineExecutionAdapter;

/// Perfect-fill paper adapter.
///
/// Every order is confirmed instantly with `filled_amount == intent.amount`
/// and zero gas. Useful for rapid prototyping but useless for honest PnL.
#[derive(Clone)]
pub struct PaperExecutionAdapter {
    order_prefix: Arc<str>,
    next_order_id: Arc<AtomicU64>,
}

impl PaperExecutionAdapter {
    pub fn new() -> Self {
        Self::with_order_prefix(unique_paper_order_prefix())
    }

    pub fn with_order_prefix(prefix: impl Into<String>) -> Self {
        Self {
            order_prefix: Arc::<str>::from(prefix.into()),
            next_order_id: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl Default for PaperExecutionAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EngineExecutionAdapter for PaperExecutionAdapter {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        let order_seq = self.next_order_id.fetch_add(1, Ordering::Relaxed) + 1;
        Ok(ExecutionReport {
            order_id: OrderId(format!("{}-{order_seq}", self.order_prefix)),
            status: ExecutionStatus::Confirmed,
            tx_hash: None,
            block_number: None,
            filled_amount: Some(intent.amount),
            gas_used: Some(0),
            error: None,
        })
    }
}

fn unique_paper_order_prefix() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("paper-{}-{millis}", std::process::id())
}
