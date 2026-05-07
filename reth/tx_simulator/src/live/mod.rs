//! Live-first simulation APIs for latency-sensitive pipelines.
//!
//! Live simulation differs from regular historical simulation because the target
//! state can be ahead of the locally persisted Reth MDBX database. These APIs
//! intentionally prefer Redis chain-state overlays written by the live block
//! processor before falling back to persisted state.

mod simulator;

pub use simulator::{LiveStateSource, LiveStateStatus, LiveTxSimulator};
