//! Transaction Executor Module
//! 
//! Handles transaction construction, simulation, submission, and monitoring
//! for protective trades.

pub mod builder;

pub use builder::{TransactionBuilder, routers};