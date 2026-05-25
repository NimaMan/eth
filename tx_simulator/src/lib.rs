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
pub mod block_context;
pub mod config;
pub mod simulator;
pub mod tx_fee_parameters;
pub mod types;
pub mod single_tx {
    pub mod parallel;
    pub mod signed;
    pub mod unsigned;
}
pub mod tx_chain {
    pub mod sequential;
    pub mod signed;
    pub mod token_metadata;
    pub mod unsigned;
}
pub mod block_trace;
pub mod contract_simulation;
pub mod live;
pub mod revert;
pub mod session;
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

// Re-export main types
pub use crate::tx_chain::token_metadata::{TokenMetadataResult, TokenMetadataSimulator};
pub use live::{
    InMemoryLiveBlockStateProvider, LatestHistoricalTxSimulator, LiveBlockState, LiveStateSource,
    LiveStateStatus, LiveTxSimulator,
};
pub use session::{
    BlockReplaySession, BlockStateSession, BlockTxAdvanceProfile, BlockTxExecuteProfile,
    BlockTxStateSession, BlockTxTraceProfile, SessionStepSummary, SessionTransaction,
    SessionTransactionKind, SimulationSession, SimulationSessionOptions, SimulationSessionState,
};
pub use simulator::{RethTxSimulator, TxSimulator};
pub use single_tx::parallel::ParallelTxSimulationOptions;
pub use single_tx::signed::SignedTransaction;
pub use single_tx::unsigned::UnsignedTransaction;
pub use tx_chain::signed::SignedTxChainSimulation;
pub use tx_chain::unsigned::{ChainStateInfo, UnsignedTxChainSimulation};
pub use tx_fee_parameters::{
    GasInputs, GasTxType, SimulationGasParameters, TxFeeContext, TxGasParameters,
};
pub use types::{
    CallFrame, FeeDefaults, FullSimulationResult, ParallelTxSimulationResult, RevertContext,
    SequentialSimulationOptions, SequentialSimulationResult, SequentialTransactionResult,
    SimulationDefaults, SimulationResult, ViewCallDefaults, ViewCallOverrides, ViewFunctionResult,
};
