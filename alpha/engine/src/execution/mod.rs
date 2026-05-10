//! Execution adapters for the alpha engine.
//!
//! This module separates paper, modeled, simulated, and real execution into
//! distinct files so the boundary between pretend fills and on-chain submission
//! is explicit.
//!
//! | Adapter | Purpose |
//! |---------|---------|
//! | `PaperExecutionAdapter` | Perfect fills for rapid prototyping |
//! | `ModeledExecutionAdapter` | Worst-case fill math using pool snapshots |
//! | `SimulatedExecutionAdapter` | EVM-backed pre-flight via `tx_simulator` |
//! | `TxExecutorAdapter` | Real on-chain submission via `tx_executor` |

mod modeled;
mod paper;
mod real;
mod simulated;

pub use modeled::{ModeledExecutionAdapter, ModeledExecutionConfig};
pub use paper::PaperExecutionAdapter;
pub use simulated::{LiveSimulatedExecutionAdapter, SimulatedExecutionAdapter};

use async_trait::async_trait;
use eth_alpha_core::{error::Result, execution::ExecutionReport, order::OrderIntent};
use crate::EngineExecutionAdapter;

/// Enum wrapper so the trader binary can select an adapter at runtime.
pub enum ExecutionAdapterKind {
    Paper(PaperExecutionAdapter),
    Modeled(ModeledExecutionAdapter),
}

#[async_trait]
impl EngineExecutionAdapter for ExecutionAdapterKind {
    async fn execute(&self, intent: OrderIntent) -> Result<ExecutionReport> {
        match self {
            Self::Paper(a) => a.execute(intent).await,
            Self::Modeled(a) => a.execute(intent).await,
        }
    }
}
