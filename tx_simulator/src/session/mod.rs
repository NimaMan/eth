//! Stateful simulation and block replay sessions.
//!
//! Sessions are the high-level API for workflows that need to keep forked state warm across
//! multiple operations. Use [`SimulationSession`] for ad-hoc transaction sequences and
//! [`BlockReplaySession`] for full-block replay, tracing, and profiling.

mod block_replay;
mod block_state;
mod block_tx_state;
mod simulation;

pub use block_replay::BlockReplaySession;
pub use block_state::BlockStateSession;
pub use block_tx_state::{
    BlockTxAdvanceProfile, BlockTxExecuteProfile, BlockTxStateSession, BlockTxTraceProfile,
};
pub use simulation::{
    SessionStepSummary, SessionTransaction, SessionTransactionKind, SimulationSession,
    SimulationSessionOptions, SimulationSessionState,
};
