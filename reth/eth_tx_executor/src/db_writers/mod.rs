//! Database writers module
//! 
//! Writes all trading decisions and transaction executions to database
//! for audit trail and performance analysis

pub mod trade_logger;

pub use trade_logger::{TradeLogger, TradeEvent, ExecutionLog};