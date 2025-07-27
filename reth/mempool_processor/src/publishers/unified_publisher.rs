//! Unified Publisher Implementation
//!
//! Central publisher that handles all signal types with multipart ZMQ messaging

use zmq::{Context, Socket};
use tracing::{info, warn, error, debug};
use serde::Serialize;
use eyre::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::message_types::{UnifiedSignal, SignalType, Severity};
use super::config::PublisherConfig;

/// Statistics for the unified publisher
#[derive(Debug, Clone)]
pub struct PublisherStats {
    pub endpoint: String,
    pub enabled: bool,
    pub total_published: u64,
    pub signals_by_type: std::collections::HashMap<String, u64>,
    pub errors: u64,
    pub buffer_full_drops: u64,
}

/// Unified publisher for all signal types
pub struct UnifiedPublisher {
    context: Context,
    socket: Socket,
    config: PublisherConfig,
    enabled: bool,
    
    // Statistics
    total_published: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    buffer_full_drops: Arc<AtomicU64>,
    signals_by_type: Arc<parking_lot::RwLock<std::collections::HashMap<String, u64>>>,
}

impl UnifiedPublisher {
    /// Create a new unified publisher
    pub fn new(config: PublisherConfig) -> Result<Self> {
        info!("Initializing unified publisher on {}", config.endpoint);
        
        let context = Context::new();
        let socket = context.socket(zmq::PUB)?;
        
        // Configure socket
        socket.set_sndhwm(config.high_water_mark as i32)?;
        socket.set_linger(config.linger_ms)?;
        socket.set_sndtimeo(config.send_timeout_ms)?;
        
        // Bind to endpoint
        socket.bind(&config.endpoint)?;
        
        info!("✅ Unified publisher bound to {}", config.endpoint);
        
        Ok(Self {
            context,
            socket,
            config,
            enabled: true,
            total_published: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            buffer_full_drops: Arc::new(AtomicU64::new(0)),
            signals_by_type: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        })
    }
    
    /// Publish a unified signal
    pub fn publish(&self, signal: &UnifiedSignal) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        
        // Check severity filter
        if signal.base.severity < self.config.min_severity {
            debug!("Signal below severity threshold, skipping: {:?} < {:?}", 
                   signal.base.severity, self.config.min_severity);
            return Ok(());
        }
        
        // Check confidence filter
        if signal.base.confidence < self.config.min_confidence {
            debug!("Signal below confidence threshold, skipping: {} < {}", 
                   signal.base.confidence, self.config.min_confidence);
            return Ok(());
        }
        
        // Serialize the signal
        let json = serde_json::to_string(signal)?;
        let topic = signal.topic();
        
        // Prepare multipart message: [topic, json_data]
        let parts = vec![topic.as_bytes(), json.as_bytes()];
        
        // Send with optional non-blocking
        let flags = if self.config.non_blocking { zmq::DONTWAIT } else { 0 };
        
        match self.socket.send_multipart(&parts, flags) {
            Ok(_) => {
                self.total_published.fetch_add(1, Ordering::Relaxed);
                
                // Update per-type statistics
                {
                    let mut stats = self.signals_by_type.write();
                    *stats.entry(topic.to_string()).or_insert(0) += 1;
                }
                
                // Log periodically
                let total = self.total_published.load(Ordering::Relaxed);
                if total % 1000 == 0 {
                    info!("Published {} signals via unified publisher", total);
                }
                
                debug!("📡 Published {} signal: {}", topic, signal.base.signal_id);
                Ok(())
            }
            Err(zmq::Error::EAGAIN) => {
                self.buffer_full_drops.fetch_add(1, Ordering::Relaxed);
                warn!("Publisher buffer full, signal dropped: {}", signal.base.signal_id);
                Ok(()) // Don't treat as error
            }
            Err(e) => {
                self.errors.fetch_add(1, Ordering::Relaxed);
                error!("Failed to publish signal: {}", e);
                Err(e.into())
            }
        }
    }
    
    /// Publish a signal by constructing UnifiedSignal from components
    pub fn publish_simple<T: Serialize>(
        &self,
        signal_type: SignalType,
        tx_hash: &str,
        from_address: &str,
        token_address: &str,
        data: T,
        details: &str,
    ) -> Result<()> {
        let signal_data = super::message_types::SignalData::Generic(
            serde_json::from_value(serde_json::to_value(data)?)?
        );
        
        let signal = UnifiedSignal::new(
            signal_type,
            tx_hash.to_string(),
            from_address.to_string(),
            token_address.to_string(),
            signal_data,
            details.to_string(),
        );
        
        self.publish(&signal)
    }
    
    /// Enable or disable publishing
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        info!("Unified publisher {}", if enabled { "enabled" } else { "disabled" });
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> PublisherStats {
        PublisherStats {
            endpoint: self.config.endpoint.clone(),
            enabled: self.enabled,
            total_published: self.total_published.load(Ordering::Relaxed),
            signals_by_type: self.signals_by_type.read().clone(),
            errors: self.errors.load(Ordering::Relaxed),
            buffer_full_drops: self.buffer_full_drops.load(Ordering::Relaxed),
        }
    }
    
    /// Clear statistics
    pub fn reset_stats(&self) {
        self.total_published.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.buffer_full_drops.store(0, Ordering::Relaxed);
        self.signals_by_type.write().clear();
    }
}

// Note: Global publisher removed due to ZMQ Context/Socket not being Send+Sync
// Users should create and manage their own UnifiedPublisher instances

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publishers::message_types::*;
    
    #[test]
    fn test_publisher_creation() {
        let config = PublisherConfig {
            endpoint: "tcp://127.0.0.1:15555".to_string(), // Test port
            high_water_mark: 1000,
            linger_ms: 0,
            send_timeout_ms: 1000,
            non_blocking: true,
            min_severity: Severity::Low,
            min_confidence: 0.0,
        };
        
        // Should create successfully
        let publisher = UnifiedPublisher::new(config);
        assert!(publisher.is_ok());
    }
    
    #[test]
    fn test_severity_filtering() {
        let config = PublisherConfig {
            endpoint: "tcp://127.0.0.1:15556".to_string(),
            high_water_mark: 1000,
            linger_ms: 0,
            send_timeout_ms: 1000,
            non_blocking: true,
            min_severity: Severity::High,
            min_confidence: 0.0,
        };
        
        let publisher = UnifiedPublisher::new(config).unwrap();
        
        // Create a low severity signal
        let mut signal = UnifiedSignal::new(
            SignalType::TaxChange,
            "0x123".to_string(),
            "0xabc".to_string(),
            "0xtoken".to_string(),
            SignalData::Generic(std::collections::HashMap::new()),
            "Test".to_string(),
        );
        signal.base.severity = Severity::Low;
        
        // Should not error but won't actually publish
        assert!(publisher.publish(&signal).is_ok());
        
        // Stats should show no publications
        let stats = publisher.get_stats();
        assert_eq!(stats.total_published, 0);
    }
}