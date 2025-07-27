/// Mempool Processor Library
/// 
/// High-performance Ethereum mempool monitoring and transaction analysis system.
/// Provides real-time transaction simulation, state change detection, and market event detection.

pub mod mempool_fetcher;
pub mod token_tracking;
// pub mod signal_engine; // Temporarily disabled until fixed
pub mod common;
pub mod config;
pub mod database;
pub mod token_parameter_extraction;
pub mod publishers;

// Re-export commonly used types
pub use mempool_fetcher::{FullTransactionIpcClient, FullTransaction, IpcClientStats};

// Temporary stub module for tx_simulator until signal_engine is fixed
pub mod tx_simulator {
    // Re-export reth_tx_simulator types directly for now
    pub use reth_tx_simulator::{
        DirectTxSimulator,
        CallRequest,
        AddressStateChange,
        BatchSimulationOptions,
        BatchSimulationResult,
        ipc_to_call_request,
        DirectTxSimulator as RethDirectTxSimulator,
    };
    
    // Re-export tx_processor if available
    pub use tx_processor::{TxProcessor, ProcessedTransaction};
}

