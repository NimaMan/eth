/// Ethereum Mempool Fetcher Module
/// 
/// This module provides multiple methods for detecting transactions in the Ethereum mempool,
/// each with different performance characteristics and use cases.
/// 
/// Available methods:
/// - IPC-IPC: 1.2ms - Unix socket subscription + fetch (current implementation)
/// - IPC-IPC Variants: Various - Legacy IPC implementations (basic, full, batch)
/// - WebSocket: 1-50ms - WebSocket subscription + HTTP/IPC fetch
/// - Direct Reth: <1ms target - ExEx integration (requires custom build)
/// 
/// HTTP RPC is NOT supported due to fatal limitations (only 7% coverage)

// Core types used by all methods
pub mod types;

// Detection methods (each in its own directory)
pub mod ipc_ipc;           // IPC subscription + IPC fetch (1.2ms baseline - current implementation)
pub mod ipc_ipc_variants;  // IPC subscription + IPC fetch (legacy variants: basic, full, batch)
pub mod websocket;         // WebSocket streaming

// Processing components
pub mod processor;      // Transaction processing logic

// Re-export main types for convenience
pub use types::*;
pub use ipc_ipc::{IpcIpcMeasurementClient, TransactionLatencyMeasurement, MeasurementConfig};
// Basic IpcClient removed due to poor performance
pub use websocket::WebSocketClient;
pub use processor::{TransactionProcessor, PoolTracker, ScamPredictionWriter};

// Re-export tx_simulator functionality 
pub use crate::tx_simulator::*;

// Tests are in the individual submodules