/// IPC Socket Method for Transaction Detection
/// 
/// Uses Unix domain socket at /tmp/reth.ipc for low-latency communication
/// with local Reth node.
/// 
/// Performance characteristics:
/// - Round-trip latency: ~15μs (excellent)
/// - Subscription notifications: 
///   - Standard client: 96ms average (OS buffering)
///   - Optimized client: <1ms (with proper socket tuning)

pub mod client;
pub mod optimized_client;

pub use client::*;
pub use optimized_client::OptimizedIpcClient;