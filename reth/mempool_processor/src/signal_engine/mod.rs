// signal_engine/mod.rs
//
// Module for analyzing mempool transactions to detect market signals,
// including scams, liquidity changes, supply anomalies, and trading opportunities.

pub mod types;
pub mod function_detector;
pub mod tx_router;
pub mod simulator;
pub mod signal_detector;
pub mod signal_generator;
pub mod signal_publisher;

// Re-export key types
pub use types::*;
pub use function_detector::FunctionDetector;
pub use signal_publisher::{SignalPublisher, SignalPublisherConfig};

// Re-export simulation types
pub use simulator::{
    TxSimulator,
    BatchProcessor,
    TxSimulationResult,
    StateChangeResult,
    CallTraceResult,
    mempool_tx_to_call_request,
};
pub use signal_detector::{SignalDetector, SignalDetectionConfig, SimulationSignal};