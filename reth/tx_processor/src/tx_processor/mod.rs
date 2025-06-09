// Transaction processor module - high-performance transaction processing using REVM

pub mod processor;
pub mod types;
pub mod rpc_processor;

// Re-exports
pub use processor::{RevmTxProcessor, ProcessorConfig};
pub use types::{ProcessedTransaction, InternalTransfer, ProcessingMetrics};
pub use rpc_processor::{RpcProcessor, RpcProcessorConfig};