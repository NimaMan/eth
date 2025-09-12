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
// Block tracing
pub mod block_trace;
// Back-compat module paths for external crates relying on previous layout
pub mod unsigned_tx_simulator { pub use crate::single_tx::unsigned::*; }
pub mod signed_tx_simulator { pub use crate::single_tx::signed::*; }
pub mod unsigned_tx_chain_simulator { pub use crate::tx_chain::unsigned::*; }
pub mod signed_tx_chain_simulator { pub use crate::tx_chain::signed::*; }
pub mod unsigned_tx_bundle_simulator { pub use crate::tx_chain::bundle::*; }
pub mod parallel_tx_simulator { pub use crate::tx_parallel::*; }
pub mod block_simulation { pub use crate::block_trace::types::*; pub use crate::block_trace::block_tracer::*; }

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
