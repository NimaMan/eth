//! ETH Kartal - High-Performance Transaction Execution Engine
//! 
//! Ultra-fast transaction execution system for Ethereum mainnet
//! designed for sub-200ms alert-to-execution latency.

pub mod alert_processor;
pub mod common;
pub mod config;
pub mod flashbots;
pub mod logging;
pub mod pools;
pub mod risk;
pub mod tx_executor;
pub mod wallet;

// Re-export commonly used types
pub use alert_processor::{Alert, AlertReceiver};
pub use common::{KartalError, Result};
pub use config::Config;
pub use logging::{TradeLogger, TradeEvent};
// Re-export ranking types from tx_ranking_system
pub use tx_ranking_system::{TransactionRankingSystem, RankingResult};
pub use tx_executor::TransactionExecutor;