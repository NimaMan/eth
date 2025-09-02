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
pub mod signed_simulation;
pub mod unsigned_simulation;
pub mod call_simulator;
pub mod unsigned_tx_bundle_simulation;
pub mod view_function_simulator;
pub mod parallel_tx_simulation;
pub mod unsigned_tx_chain_simulation;
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
pub use call_simulator::UnsignedTransaction;
pub use signed_simulation::SignedTransaction;
pub use unsigned_tx_chain_simulation::{UnsignedTxChainSimulation, ChainStateInfo};
pub use parallel_tx_simulation::ParallelTxSimulationOptions;