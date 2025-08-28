/// Transaction Simulator Library
///
/// This library provides high-performance Ethereum transaction simulation
/// using direct Reth database access. It focuses purely on simulation:
/// - Signed transaction simulation (from mempool or RPC)
/// - Unsigned transaction simulation (like debug_traceCall)
/// - Sequential transaction simulation (MEV bundles)
/// - View function calls
/// - Batch transaction simulation
/// - Returns raw CallFrame traces for tx_processor to analyze

pub mod types;
pub mod simulator;
pub mod signed_simulation;
pub mod unsigned_simulation;
pub mod call_simulator;
pub mod batch_sequence_simulation;
pub mod view_function_simulator;
pub mod parallel_tx_simulation;
pub mod simulation_chain;

// Re-export main types
pub use simulator::{TxSimulator, RethTxSimulator};
pub use types::{
    SimulationResult,
    FullSimulationResult,
    ViewFunctionResult,
    BatchSimulationResult,
    SequentialSimulationResult,
    SequentialTransactionResult,
    SequentialSimulationOptions,
    CallFrame,
};
pub use call_simulator::CallRequest;
pub use simulation_chain::{SimulationChain, ChainStateInfo};