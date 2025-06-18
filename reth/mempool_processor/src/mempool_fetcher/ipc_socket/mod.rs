/// IPC Socket Method for Transaction Detection
/// 
/// Uses Unix domain socket at /tmp/reth.ipc for low-latency communication
/// with local Reth node.
/// 
/// Performance characteristics:
/// - Round-trip latency: ~15μs (excellent)
/// - Subscription notifications: 
///   - Optimized client: <1ms (with proper socket tuning)
///   - Full TX client: ~10-20ms (includes full transaction data)
///   - Batch TX client: High throughput with connection pooling
/// 
/// Available implementations:
/// - `OptimizedIpcClient`: Low-latency single transaction processing
/// - `FullTxIpcClient`: Gets full transaction data in subscription
/// - `BatchTxIpcClient`: High-throughput batch processing

pub mod client;
pub mod optimized_client;
pub mod full_tx_client;
pub mod batch_tx_client;

pub use client::*;
pub use optimized_client::OptimizedIpcClient;
pub use full_tx_client::{FullTxIpcClient, FullIpcTransaction};
pub use batch_tx_client::{BatchTxIpcClient, BatchConfig, BatchStats};