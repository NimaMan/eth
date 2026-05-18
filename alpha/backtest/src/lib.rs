//! Historical replay backtest crate for the Ethereum alpha trading system.
//!
//! Replays `MarketEvent` and `RiskEvent` streams through the same `AlphaEngine`
//! used by live trading, using EVM-backed swap simulation for chain parity.
//! All fills are produced by running actual swap calldata through the EVM at
//! the historical block — no model-based estimation.

pub mod adapter;
pub mod config;
pub mod execution;
pub mod replay;
pub mod runner;
pub mod strategy_suites;
