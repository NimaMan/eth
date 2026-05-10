//! Historical replay backtest crate for the Ethereum alpha trading system.
//!
//! Replays `MarketEvent` and `RiskEvent` streams through the same `AlphaEngine`
//! used by live trading, using a `SimulatedExecutionAdapter` that models fill
//! prices, slippage, gas costs, and transaction failures.

pub mod config;
pub mod execution;
pub mod runner;
