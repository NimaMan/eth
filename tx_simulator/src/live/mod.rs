//! Live-first simulation APIs for latency-sensitive pipelines.
//!
//! Live simulation differs from regular historical simulation because the target
//! state can be ahead of the local Reth context that is readable by the
//! simulator. These APIs use local historical context when it is caught up and
//! otherwise use state tracked by the live block processor.

mod simulator;

pub use simulator::{LiveStateSource, LiveStateStatus, LiveTxSimulator};
