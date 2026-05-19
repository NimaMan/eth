//! Execution adapters for the alpha engine.
//!
//! Runtime evaluation uses chain-state simulation only. Snapshot-price and
//! perfect-fill adapters are intentionally not part of the pipeline because they
//! produce theoretical PnL instead of chain-parity fills.
//!
//! | Adapter | Purpose |
//! |---------|---------|
//! | `ChainSimExecutionAdapter` | Historical backtest simulation via `tx_simulator` |
//! | `LiveChainSimExecutionAdapter` | Live no-capital simulation via `LiveTxSimulator` |
//! | `TxExecutorAdapter` | Real on-chain submission via `tx_executor` |

mod real;
mod simulated;

pub use real::{
    LiveTradingPlannerBridge, LiveTxPlanner, LiveTxPlanningInputResolver, LiveTxSubmitter,
    TxExecutorAdapter,
};
pub use simulated::{ChainSimExecutionAdapter, LiveChainSimExecutionAdapter};
