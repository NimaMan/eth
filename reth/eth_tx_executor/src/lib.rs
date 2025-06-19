//! ETH Kartal - Automated Trading Protection System
//! 
//! A high-performance system for protecting users from scam transactions
//! by executing protective trades in response to real-time alerts.

pub mod alert_processor;
pub mod common;
pub mod risk;
pub mod strategy;
pub mod tx_executor;
pub mod wallet;

// Re-export commonly used types
// pub use alert_processor::{ScamAlert, AlertReceiver};
// pub use strategy::{Strategy, DecisionEngine};
// pub use tx_executor::TransactionExecutor;