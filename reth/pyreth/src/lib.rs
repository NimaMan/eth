/// PyReth - Python bindings for Reth-based Ethereum tools
/// 
/// This crate provides a unified Python interface for:
/// - Direct blockchain queries (ChainQuery)
/// - Transaction processing (TxProcessor)
/// - Transaction simulation (Simulator)
/// - Transaction building (TxBuilder)
/// 
/// All functionality is accessible through a single Python module: `import pyreth`

// Core modules
pub mod trading_simulator;

// Re-export external functionality
pub use tx_processor::{TxProcessor, ProcessedTransaction};
pub use reth_tx_simulator::{RethTxSimulator, CallRequest, SimulationResult};

// Re-export erc20_token_trading_viability from tx_processor
pub use tx_processor::erc20_token_trading_viability;

#[cfg(feature = "python")]
pub use tx_builder::TxBuilder;

// Python bindings (only when python feature is enabled)
#[cfg(feature = "python")]
pub mod python;

// Utility modules
pub mod utils {
    pub use tx_processor::utils::*;
}