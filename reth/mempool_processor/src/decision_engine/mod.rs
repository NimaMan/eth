// decision_engine/mod.rs
//
// Module for analyzing mempool transactions to detect market events,
// including scams, liquidity changes, supply anomalies, and trading opportunities.

pub mod types;
pub mod engine;
pub mod service;

// Re-export key types
pub use types::*;
pub use engine::{DecisionEngine, DecisionConfig};
pub use service::{DecisionService, ServiceStats};

// Keep legacy naming for backward compatibility during migration
pub use engine::DecisionEngine as ScamDetectionEngine;
pub use engine::DecisionConfig as ScamDetectionConfig;
pub use service::DecisionService as ScamDetectionService;