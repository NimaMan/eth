//! Execution adapters for the alpha engine.
//!
//! Runtime evaluation uses chain-state simulation only. Snapshot-price and
//! perfect-fill adapters are intentionally not part of the pipeline because they
//! produce theoretical PnL instead of chain-parity fills.
//!
//! | Adapter | Purpose |
//! |---------|---------|
//! | `ChainSimExecutionAdapter` | Historical backtest simulation via `tx_simulator` |
//! | `LiveChainSimExecutionAdapter` | Live no-capital submission plus exact-block in-memory chain-sim settlement |
//! | `TxExecutorAdapter` | Crate-private real live submission via Kartal |

pub(crate) mod real;
mod sell_economics;
mod simulated;

pub use simulated::{ChainSimExecutionAdapter, LiveChainSimExecutionAdapter};
