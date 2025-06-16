// signal_engine/mod.rs
//
// Module for analyzing mempool transactions to detect market signals,
// including scams, liquidity changes, supply anomalies, and trading opportunities.

pub mod types;
pub mod engine;
pub mod service;

// Re-export key types
pub use types::*;
pub use engine::{SignalEngine, SignalConfig};
pub use service::{SignalService, ServiceStats};

// Keep legacy naming for backward compatibility during migration
pub use engine::SignalEngine as ScamDetectionEngine;
pub use engine::SignalConfig as ScamDetectionConfig;
pub use service::SignalService as ScamDetectionService;

// Also keep decision_engine aliases for smoother migration
pub use engine::SignalEngine as DecisionEngine;
pub use engine::SignalConfig as DecisionConfig;
pub use service::SignalService as DecisionService;