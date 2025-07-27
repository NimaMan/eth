// signal_engine/mod.rs
//
// Module for analyzing mempool transactions to detect market signals,
// including scams, liquidity changes, supply anomalies, and trading opportunities.

pub mod types;
pub mod engine;
pub mod service;
pub mod function_detector;
pub mod pool_analyzer;
pub mod creator_analyzer;
pub mod token_parameter_extractor;
pub mod tx_router;
pub mod simulator;
pub mod detectors;
pub mod signal_generator;

// Re-export key types
pub use types::*;
pub use engine::{SignalEngine, SignalConfig};
pub use service::{SignalService, ServiceStats};
pub use function_detector::FunctionDetector;
pub use pool_analyzer::{PoolAnalyzer, PoolAnalysisConfig};
pub use creator_analyzer::{CreatorAnalyzer, CreatorAlert, AlertSeverity};

// Re-export simulation types
pub use simulator::{
    TxSimulator,
    BatchProcessor,
    TxSimulationResult,
    StateChangeResult,
    CallTraceResult,
    mempool_tx_to_call_request,
};
pub use detectors::{SignalDetector, SignalDetectionConfig, SimulationSignal};

// Keep legacy naming for backward compatibility during migration
pub use engine::SignalEngine as ScamDetectionEngine;
pub use engine::SignalConfig as ScamDetectionConfig;
pub use service::SignalService as ScamDetectionService;

// Also keep decision_engine aliases for smoother migration
pub use engine::SignalEngine as DecisionEngine;
pub use engine::SignalConfig as DecisionConfig;
pub use service::SignalService as DecisionService;