/// Clean TX Processor - Rust alternative to Python eth_block_processor
/// 
/// This is a simplified, clean transaction processor that replaces the complex
/// Python eth_block_processor.txn module with direct Reth database access.
/// 
/// Key improvements over Python version:
/// - 10-40x faster (direct DB vs RPC)
/// - Much simpler codebase
/// - No complex RPC handling
/// - Consistent performance

// Re-export the NEW TX Simulator (migrated from reth_tx_simulator)
pub use tx_simulator::{
    TxSimulator,
    CallRequest, 
    SimulationResult,
    FullSimulationResult,
    BatchSimulationResult,
    SequentialSimulationOptions,
    SequentialSimulationResult,
};

// Keep old re-exports for backward compatibility during migration
pub use reth_tx_simulator::{
    RethTxSimulator,
    DetailedSimulationResult,
};

// Export transaction processing modules (renamed from processing to tx_processor)
pub mod tx_processor;
pub mod processed_tx_provider;

// Re-export data models from tx_processor
pub use tx_processor::data_models::{ProcessedTransaction, TransactionFees};
pub mod config;
pub mod utils;
pub mod retry_utils;
// Chain query moved to external crate reth_chain_query
// pub mod chain_query;

// Export ERC20 token trading viability module
// TODO: Re-enable after fixing sequential simulation dependencies
// pub mod erc20_token_trading_viability;
// pub use erc20_token_trading_viability::{
//     OptionalSetupBuyApproveSellTokenSimulator,
//     OptionalSetupBuyApproveSellResult,
// };


// Export new modular structure
pub mod sequential_tx_simulator;

// Python bindings module (only included when building for Python)

// OLD TX Processor functionality removed - use core::TxProcessor instead
