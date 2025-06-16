/// Ethereum Mempool Fetcher Module
/// 
/// This module provides multiple methods for detecting transactions in the Ethereum mempool,
/// each with different performance characteristics and use cases.
/// 
/// Available methods:
/// - IPC Socket: 20μs-300ms - Unix socket connection to local Reth
/// - WebSocket: 1-50ms - Standard method with full coverage
/// - DevP2P: <10ms target - Direct P2P protocol (in development)
/// - Direct Reth: <1ms target - ExEx integration (requires custom build)
/// 
/// HTTP RPC is NOT supported due to fatal limitations (only 7% coverage)

// Core types used by all methods
pub mod types;

// Detection methods (each in its own directory)
pub mod ipc_socket;     // Unix domain socket connection
pub mod websocket;      // WebSocket streaming
pub mod devp2p;         // Direct P2P protocol

// Processing components
pub mod processor;      // Transaction processing logic

// Re-export main types for convenience
pub use types::*;
pub use ipc_socket::IpcClient;
pub use websocket::WebSocketClient;
pub use devp2p::DevP2pClient;
pub use processor::{TransactionProcessor, PoolTracker, DbLogger};

// Re-export tx_simulator functionality 
pub use crate::tx_simulator::*;

// Tests are in the individual submodules