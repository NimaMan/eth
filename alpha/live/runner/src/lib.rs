//! Live alpha runner crate.
//!
//! This crate owns the live trading binaries and the `live_trader/` runtime
//! subtree. It depends on `eth_alpha_engine` for the backtest-safe core
//! (`AlphaEngine`, execution adapters, wire types, valuation) and isolates all
//! live wiring — real submission, receipt reconciliation, operator controls,
//! chain-sim settlement, and the live poll loop — so the engine remains a pure
//! backtest-safe library that cannot accidentally import live execution.
//!
//! ## Algorithmic overview
//! The live runner drives a single poll loop (`live_trader::run`). Each tick it:
//! 1. reads a live block frame + mempool signals from the chain server,
//! 2. settles prior chain-sim executions and reconciles real receipts,
//! 3. processes pool updates and mempool signals into engine events,
//! 4. runs operator manual-close requests and the position monitor,
//! 5. values open positions for snapshot parity, and
//! 6. emits a heartbeat, then waits for the next frame.
//! `run_live_real` and `run_live_backtest` are the two entrypoints, selecting
//! the real-executor and no-capital chain-sim execution modes respectively.

// The live-trader heartbeat builds a large `json!` metadata object; the default
// macro recursion limit (128) is exceeded once enough keys are present.
#![recursion_limit = "256"]

pub mod live_trader;

pub use live_trader::{run_live_backtest, run_live_real};
