// scam_detection/mod.rs
//
// Module for detecting potential scam transactions by comparing
// mempool transaction state diffs against current pool ETH levels.

pub mod types;
pub mod engine;
pub mod service;

// Re-export key types
pub use types::*;
pub use engine::{ScamDetectionEngine, ScamDetectionConfig};
pub use service::{ScamDetectionService, ServiceStats};