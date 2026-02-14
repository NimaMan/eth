//! Database writers module
//!
//! Writes trading decisions and transaction executions to database

pub mod trade_logger;

pub use trade_logger::{TradeEvent, TradeLogger};
