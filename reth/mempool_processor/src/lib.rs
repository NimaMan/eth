/// Mempool Processor Library
/// 
/// High-performance Ethereum mempool monitoring and transaction analysis system.
/// Provides real-time transaction simulation, state change detection, and market event detection.

pub mod mempool_fetcher;
pub mod token_tracking;
pub mod common;
pub mod config;
pub mod database;
pub mod token_parameter_extraction;
pub mod publishers;

// Signal processing modules
pub mod function_detector;
pub mod signal_detector;
pub mod signal_generator;
pub mod signal_publisher;
pub mod simulator;
pub mod tx_router;

// Re-export commonly used types
pub use mempool_fetcher::{FullTransactionIpcClient, FullTransaction, IpcClientStats};

// Temporary stub module for tx_simulator until signal_engine is fixed
pub mod tx_simulator {
    // Re-export reth_tx_simulator types directly for now
    pub use reth_tx_simulator::{
        RethTxSimulator,
        CallRequest,
        AddressStateChange,
        BatchSimulationOptions,
        BatchSimulationResult,
        ipc_to_call_request,
    };
    
    // Re-export tx_processor if available
    pub use tx_processor::{TxProcessor, ProcessedTransaction};
}

