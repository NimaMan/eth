//! Trade logging module
//! 
//! Provides comprehensive logging of all trading decisions and executions
//! for audit trail and performance analysis

pub mod trade_logger;

pub use trade_logger::{TradeLogger, TradeEvent, ExecutionLog};