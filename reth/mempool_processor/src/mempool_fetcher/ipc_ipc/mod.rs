//! IPC-IPC Mempool Transaction Measurement
//! 
//! High-performance transaction timing using Unix IPC socket for both
//! subscription and transaction fetching.
//! 
//! Achieves 1.201ms average latency with 43.3% sub-millisecond performance.

pub mod measurement_client;

pub use measurement_client::{
    IpcIpcMeasurementClient,
    TransactionLatencyMeasurement, 
    MeasurementConfig,
    MeasurementResults,
};

/// Default IPC socket path for Reth node
pub const DEFAULT_IPC_PATH: &str = "/tmp/reth.ipc";

/// Default measurement configuration
pub fn default_config() -> MeasurementConfig {
    MeasurementConfig {
        ipc_socket_path: DEFAULT_IPC_PATH.to_string(),
        target_transaction_count: 1000,
        log_directory: "/home/nima/code/crypto/logs/mempool_fetch".to_string(),
        progress_interval_transactions: 100,
        high_latency_threshold_ms: 10.0,
    }
}