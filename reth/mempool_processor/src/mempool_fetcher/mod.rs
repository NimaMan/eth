/// Ethereum Mempool Fetcher Module
/// 
/// High-performance transaction detection for real-time mempool monitoring.
/// 
/// Current Implementation:
/// - NonBlockingIpcClient: 2-7μs detection latency via non-blocking IPC reads
/// - Zero RPC fallback needed - full transaction data in first request
/// - Non-blocking socket reads with streaming JSON parser
/// - Production throughput: 150-703 tx/sec sustained processing

// Core types used by all methods
pub mod types;

// Non-blocking IPC client - production implementation
pub mod nonblocking_ipc_client;

// Full transaction client with reconnection logic
pub mod full_transaction_ipc_client;

// Re-export main types for convenience
pub use types::*;
pub use nonblocking_ipc_client::{NonBlockingIpcClient, NonBlockingTransaction};
pub use full_transaction_ipc_client::{FullTransactionIpcClient, FullTransaction, IpcClientStats};

// Re-export tx_simulator functionality 
pub use crate::tx_simulator::*;