// Transaction processor module - high-performance transaction processing using REVM

pub mod processor;
pub mod processor_v2;
pub mod types;
pub mod rpc_processor;

// Re-exports - use v2 processor
pub use processor_v2::{RevmTxProcessor, ProcessorConfig};
pub use types::{ProcessedTransaction, InternalTransfer, ProcessingMetrics};
pub use rpc_processor::{RpcProcessor, RpcProcessorConfig};