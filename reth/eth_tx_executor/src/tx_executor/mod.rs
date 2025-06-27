//! Transaction Executor Module
//! 
//! High-performance transaction execution with sub-200ms latency target

pub mod builder;
pub mod executor;

pub use builder::{TransactionBuilder, routers};
pub use executor::{TransactionExecutor, ExecutorConfig, ExecutionResult, ExecutionMetrics};