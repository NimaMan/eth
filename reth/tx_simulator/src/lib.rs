pub mod block_context;
pub mod config;
pub mod gas;
pub mod simulator;
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
// New structured modules
pub mod single_tx {
    pub mod parallel;
    pub mod signed;
    pub mod unsigned;
}
pub mod tx_chain {
    pub mod sequential;
    pub mod signed;
    pub mod unsigned;
}
pub mod block_trace;
pub mod contract_method_simulator;
pub mod header_utils;
pub mod live_chain_data;
pub mod simulation_revert_decoder;
pub mod tx_builders;

// Back-compat module paths for external crates relying on previous layout
pub mod unsigned_tx_simulator {
    pub use crate::single_tx::unsigned::*;
}
pub mod signed_tx_simulator {
    pub use crate::single_tx::signed::*;
}
pub mod unsigned_tx_chain_simulator {
    pub use crate::tx_chain::unsigned::*;
}
pub mod signed_tx_chain_simulator {
    pub use crate::tx_chain::signed::*;
}

pub mod parallel_tx_simulator {
    pub use crate::single_tx::parallel::*;
}
pub mod block_simulation {
    pub use crate::block_trace::block_tracer::*;
    pub use crate::block_trace::types::*;
}

pub mod live_chain_cache {
    pub use crate::live_chain_data::live_chain_cache::*;
}

// Re-export main types
pub use gas::{
    GasHeuristic, GasInputs, GasResolutionContext, GasTxType, ResolvedGasParameters,
    TxGasParameters,
};
pub use live_chain_cache::{LiveChainCache, LiveChainCacheBuilder};
pub use simulator::{RethTxSimulator, TxSimulator};
pub use single_tx::parallel::ParallelTxSimulationOptions;
pub use single_tx::signed::SignedTransaction;
pub use single_tx::unsigned::UnsignedTransaction;
pub use tx_chain::signed::SignedTxChainSimulation;
pub use tx_chain::unsigned::{ChainStateInfo, UnsignedTxChainSimulation};
pub use types::{
    CallFrame, FeeDefaults, FullSimulationResult, ParallelTxSimulationResult, RevertContext,
    SequentialSimulationOptions, SequentialSimulationResult, SequentialTransactionResult,
    SimulationDefaults, SimulationResult, ViewCallDefaults, ViewCallOverrides, ViewFunctionResult,
};
