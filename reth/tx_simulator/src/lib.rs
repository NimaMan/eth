/// Transaction Simulator Library
///
/// This library provides high-performance Ethereum transaction simulation
/// using direct Reth database access. It focuses purely on simulation:
/// - Signed transaction simulation (from mempool or RPC)
/// - Unsigned transaction simulation (UnsignedTransaction - like debug_traceCall)
/// - Sequential transaction simulation (MEV bundles)
/// - View function calls
/// - Batch transaction simulation
/// - Returns raw CallFrame traces for tx_processor to analyze

pub mod types;
pub mod simulator;
pub mod signed_tx_simulator;
pub mod unsigned_tx_simulator;
pub mod unsigned_tx_bundle_simulator;
pub mod contract_method_simulator;
pub mod parallel_tx_simulator;
pub mod unsigned_tx_chain_simulator;
pub mod signed_tx_chain_simulator;
pub mod simulation_revert_decoder;
// Block simulation module for tracing entire blocks
pub mod block_simulation;

// Re-export main types
pub use simulator::{TxSimulator, RethTxSimulator};
pub use types::{
    SimulationResult,
    FullSimulationResult,
    ViewFunctionResult,
    ParallelTxSimulationResult,
    SequentialSimulationResult,
    SequentialTransactionResult,
    SequentialSimulationOptions,
    CallFrame,
};
pub use unsigned_tx_simulator::UnsignedTransaction;
pub use signed_tx_simulator::SignedTransaction;
pub use unsigned_tx_chain_simulator::{UnsignedTxChainSimulation, ChainStateInfo};
pub use signed_tx_chain_simulator::SignedTxChainSimulation;
pub use parallel_tx_simulator::ParallelTxSimulationOptions;
