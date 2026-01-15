//! ETH Kartal - High-Performance Transaction Execution Engine
//! 
//! Ultra-fast transaction execution system for Ethereum mainnet
//! designed for sub-200ms alert-to-execution latency.

pub mod alert_processor;
pub mod common;
pub mod config;
pub mod flashbots;
pub mod gas_ranking;
pub mod db_writers;
pub mod pools;
pub mod risk;
pub mod tx_executor;
pub mod wallet;

// Re-export commonly used types
pub use alert_processor::{Alert, AlertReceiver};
pub use common::{KartalError, Result};
pub use config::Config;
pub use db_writers::{TradeLogger, TradeEvent};
pub use tx_executor::TransactionExecutor;