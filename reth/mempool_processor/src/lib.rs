/// Mempool Processor Library
/// 
/// High-performance Ethereum mempool monitoring and transaction analysis system.
/// Provides real-time transaction simulation, state change detection, and market event detection.

pub mod mempool_fetcher;
pub mod token_tracking;
pub mod common;
pub mod config;
#[cfg(feature = "db")]
pub mod db_writers;

// Signal processing modules
pub mod function_detector;
pub mod signal_detector;
pub mod signal_publisher;
pub mod simulator;
pub mod tx_router;
pub mod arrival_recorder;

// Re-export commonly used types  
// Note: Legacy FullTransactionIpcClient removed, use MempoolFetcherIPCClient instead

// Migration note: tx_simulator functionality has been migrated to tx_processor
// The old reth_tx_simulator is no longer used
/* 
pub mod tx_simulator {
    // Re-export reth_tx_simulator types directly for now
    pub use reth_tx_simulator::{
        RethTxSimulator,
        CallRequest,
        AddressStateChange,
        BatchSimulationOptions,
        BatchSimulationResult,
    };
    
    // Re-export tx_processor if available
    // pub use tx_processor::{TxProcessor, ProcessedTransaction};
}
*/
