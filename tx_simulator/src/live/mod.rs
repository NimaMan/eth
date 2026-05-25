//! Live simulation APIs for latency-sensitive pipelines.
//!
//! `LiveTxSimulator` is backed only by in-memory block sessions published by
//! the live block processor. Latest-Reth historical simulation uses
//! `LatestHistoricalTxSimulator` explicitly.

mod simulator;

pub use simulator::{
    InMemoryLiveBlockStateProvider, LatestHistoricalTxSimulator, LiveBlockState, LiveStateSource,
    LiveStateStatus, LiveTxSimulator,
};
