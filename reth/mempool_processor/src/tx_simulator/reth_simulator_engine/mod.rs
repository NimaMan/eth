/// Direct Reth Simulator Engine
/// 
/// High-performance transaction simulation engine that bypasses RPC entirely
/// by directly accessing Reth's MDBX database and REVM execution engine.
/// 
/// Performance: 100-250x faster than RPC simulation
/// - Simple transfers: ~400µs
/// - Complex transactions: ~2.8ms

pub mod simulator;
pub mod tx_converter;
pub mod state_change_extractor;
pub mod simulator_with_state_changes;

pub use simulator::{RethDirectSimulator, SimulationResult};
pub use tx_converter::mempool_tx_to_reth_signed;
pub use state_change_extractor::StateChangeExtractor;
pub use simulator_with_state_changes::RethDirectSimulatorWithStateChanges;