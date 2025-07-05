/// Ethereum Mempool Fetcher Module
/// 
/// Ultra-fast transaction detection for real-time mempool monitoring.
/// 
/// Current Implementation:
/// - UltraFastClient: 2-7μs detection latency via direct IPC integration
/// - Zero RPC fallback needed - full transaction data in first request
/// - Non-blocking socket reads with streaming JSON parser
/// - Production throughput: 150-703 tx/sec sustained processing

// Core types used by all methods
pub mod types;

// Ultra-fast IPC client - production implementation
pub mod ultra_fast_client;

// Legacy full transaction client (backup)
pub mod full_transaction_ipc_client;

// Re-export main types for convenience
pub use types::*;
pub use ultra_fast_client::{UltraFastClient, UltraFastTransaction};
pub use full_transaction_ipc_client::{FullTransactionIpcClient, FullTransaction, IpcClientStats};

// Re-export tx_simulator functionality 
pub use crate::tx_simulator::*;