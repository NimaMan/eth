//! Publisher Configuration

use serde::{Serialize, Deserialize};
use super::message_types::Severity;

/// Configuration for the unified publisher
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherConfig {
    /// ZMQ endpoint to bind to
    pub endpoint: String,
    
    /// High water mark for the socket (max queued messages)
    pub high_water_mark: usize,
    
    /// Linger time in milliseconds (time to wait on close)
    pub linger_ms: i32,
    
    /// Send timeout in milliseconds
    pub send_timeout_ms: i32,
    
    /// Use non-blocking sends
    pub non_blocking: bool,
    
    /// Minimum severity to publish
    pub min_severity: Severity,
    
    /// Minimum confidence to publish (0.0 - 1.0)
    pub min_confidence: f64,
}

impl Default for PublisherConfig {
    fn default() -> Self {
        Self {
            endpoint: "tcp://127.0.0.1:5560".to_string(), // New unified port
            high_water_mark: 10000,
            linger_ms: 0,
            send_timeout_ms: 1000,
            non_blocking: true,
            min_severity: Severity::Low,
            min_confidence: 0.5,
        }
    }
}

impl PublisherConfig {
    /// Create config for development/testing
    pub fn development() -> Self {
        Self {
            endpoint: "tcp://127.0.0.1:5560".to_string(),
            high_water_mark: 1000,
            linger_ms: 100,
            send_timeout_ms: 5000,
            non_blocking: false,
            min_severity: Severity::Low,
            min_confidence: 0.0,
        }
    }
    
    /// Create config for production
    pub fn production() -> Self {
        Self {
            endpoint: "tcp://0.0.0.0:5560".to_string(), // Listen on all interfaces
            high_water_mark: 50000,
            linger_ms: 0,
            send_timeout_ms: 100,
            non_blocking: true,
            min_severity: Severity::Medium,
            min_confidence: 0.7,
        }
    }
    
    /// Create config for high-frequency trading
    pub fn high_frequency() -> Self {
        Self {
            endpoint: "tcp://127.0.0.1:5560".to_string(),
            high_water_mark: 100000,
            linger_ms: 0,
            send_timeout_ms: 10,
            non_blocking: true,
            min_severity: Severity::High,
            min_confidence: 0.8,
        }
    }
}