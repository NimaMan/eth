// Block-wide tracing
pub mod types;
pub mod block_tracer;

// Back-compat module: re-export as block_simulation
pub mod block_simulation {
    pub use super::types::*;
    pub use super::block_tracer::*;
}
