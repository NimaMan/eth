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
// New structured modules
pub mod single_tx {
    pub mod unsigned;
    pub mod signed;
}
pub mod tx_chain {
    pub mod unsigned;
    pub mod signed;
    pub mod bundle;
}
pub mod tx_parallel;
pub mod contract_method_simulator;
pub mod simulation_revert_decoder;
// Block tracing (renamed from block_simulation)
pub mod block_trace {
    pub mod block_simulation; // keep inner name for now to minimize churn
}
// Back-compat path so imports like tx_simulator::block_simulation::* still work
pub use block_trace::block_simulation;

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
pub use single_tx::unsigned::UnsignedTransaction;
pub use single_tx::signed::SignedTransaction;
pub use tx_chain::unsigned::{UnsignedTxChainSimulation, ChainStateInfo};
pub use tx_chain::signed::SignedTxChainSimulation;
pub use tx_parallel::ParallelTxSimulationOptions;
