//! # Embedded Reth Mempool
//! 
//! Ultra-low latency Ethereum mempool transaction detection using embedded Reth networking stack.
//! 
//! ## Performance
//! 
//! | Method | Latency | Notes |
//! |--------|---------|--------|
//! | **Embedded Reth** | **Architectural improvement** | Eliminates IPC overhead |
//! | Unix IPC | 150-300 µs | Standard RPC |
//! | WebSocket | 1500-3000 µs | HTTP RPC |
//!
//! **Note**: This implementation provides architectural improvements by eliminating
//! IPC/JSON-RPC overhead, but specific performance gains require benchmarking.
//! 
//! ## Quick Start
//! 
//! ```rust,no_run
//! use embedded_reth_mempool::{EmbeddedRethConfig, EmbeddedRethListener};
//! use futures::StreamExt;
//! 
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     // Create listener
//!     let config = EmbeddedRethConfig::default();
//!     let listener = EmbeddedRethListener::new(config).await?;
//!     
//!     // Start processing
//!     listener.start_processing().await?;
//!     
//!     // Subscribe to transactions
//!     let mut tx_stream = listener.subscribe();
//!     
//!     // Process transactions with improved latency
//!     while let Some(tx) = tx_stream.next().await {
//!         println!("TX: {} | Processing time: {}μs", tx.hash_short(), tx.latency_us());
//!     }
//!     
//!     Ok(())
//! }
//! ```
//! 
//! ## Integration with mempool_processor
//! 
//! This library is designed to integrate seamlessly with the main mempool_processor:
//! 
//! ```rust,no_run
//! // In your signal detection code:
//! let config = EmbeddedRethConfig::with_port(30313);
//! let listener = EmbeddedRethListener::new(config).await?;
//! listener.start_processing().await?;
//! 
//! let mut tx_stream = listener.subscribe();
//! while let Some(tx) = tx_stream.next().await {
//!     // Your existing signal detection logic here
//!     let mempool_tx = MempoolTransaction::from(&tx);
//!     process_transaction(mempool_tx).await?;
//! }
//! ```

pub mod config;
pub mod embedded_reth;
pub mod metrics;
pub mod transaction;

// Re-export main types for convenience
pub use config::EmbeddedRethConfig;
pub use embedded_reth::{EmbeddedRethListener, TransactionStream};
pub use metrics::{TransactionMetrics, PerformanceReport};
pub use transaction::{EmbeddedTransaction, MempoolTransaction, TimingBreakdown};

/// Result type for this crate
pub type Result<T> = eyre::Result<T>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_creation() {
        let config = EmbeddedRethConfig::default();
        assert_eq!(config.listen_port, 30313);
        assert!(!config.boot_nodes.is_empty());
        assert!(!config.enable_discovery);
    }
    
    #[test]
    fn test_config_with_port() {
        let config = EmbeddedRethConfig::with_port(8080);
        assert_eq!(config.listen_port, 8080);
    }
    
    #[test]
    fn test_metrics_creation() {
        let metrics = TransactionMetrics::new();
        assert_eq!(metrics.average_latency_us(), 0.0);
        assert_eq!(metrics.tps(), 0.0);
    }
}