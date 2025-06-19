//! ZMQ Publisher for Signal Engine Events
//! 
//! Publishes market events and scam alerts to external systems like ETH Kartal

use zmq::{Context, Socket};
use serde_json;
use tracing::{info, error, warn};
use crate::signal_engine::{MarketEvent, EventType, Severity};
use eyre::Result;

/// Alert structure for external consumers
#[derive(Debug, Clone, serde::Serialize)]
pub struct AlertMessage {
    // Alert metadata
    pub alert_id: String,
    pub timestamp: u64,
    pub severity: String,
    pub event_type: String,
    
    // Transaction context
    pub tx_hash: String,
    pub detected_latency_us: u64,
    
    // Pool information
    pub pool_address: String,
    pub pool_version: String,
    pub token_address: String,
    pub token_symbol: String,
    pub token_decimals: u8,
    
    // State changes
    pub current_eth_reserve: f64,
    pub simulated_eth_reserve: f64,
    pub eth_change_amount: f64,
    pub eth_change_percent: f64,
    
    // Current prices
    pub current_price: f64,
    pub simulated_price: f64,
    pub price_impact_percent: f64,
    
    // Risk metrics
    pub confidence_score: f64,
    pub gas_price_gwei: f64,
    
    // Additional context
    pub details: String,
}

impl From<&MarketEvent> for AlertMessage {
    fn from(event: &MarketEvent) -> Self {
        // Calculate reserves and changes
        let current_eth = event.metrics.new_eth_reserve - event.metrics.eth_change;
        let simulated_eth = event.metrics.new_eth_reserve;
        let current_token = event.metrics.new_token_reserve - event.metrics.token_change;
        let simulated_token = event.metrics.new_token_reserve;
        
        // Calculate prices (token per ETH)
        let current_price = if current_eth > 0.0 {
            current_token / current_eth
        } else {
            0.0
        };
        
        let simulated_price = if simulated_eth > 0.0 {
            simulated_token / simulated_eth
        } else {
            0.0
        };
        
        let price_impact = if current_price > 0.0 {
            ((simulated_price - current_price) / current_price) * 100.0
        } else {
            0.0
        };
        
        AlertMessage {
            alert_id: format!("{}_{}", event.tx_hash.chars().take(10).collect::<String>(), event.timestamp),
            timestamp: event.timestamp,
            severity: format!("{:?}", event.severity),
            event_type: format!("{:?}", event.event_type),
            
            tx_hash: event.tx_hash.clone(),
            detected_latency_us: 0, // Will be set by receiver
            
            pool_address: event.pool_address.clone(),
            pool_version: "V2".to_string(), // TODO: Detect from pool
            token_address: event.token_address.clone(),
            token_symbol: event.metrics.token_symbol.clone(),
            token_decimals: 18, // TODO: Get from token info
            
            current_eth_reserve: current_eth,
            simulated_eth_reserve: simulated_eth,
            eth_change_amount: event.metrics.eth_change,
            eth_change_percent: event.metrics.eth_percent * 100.0,
            
            current_price,
            simulated_price,
            price_impact_percent: price_impact,
            
            confidence_score: event.confidence,
            gas_price_gwei: 30.0, // TODO: Get from transaction
            
            details: event.details.clone(),
        }
    }
}

pub struct AlertPublisher {
    context: Context,
    socket: Socket,
    endpoint: String,
    enabled: bool,
    published_count: u64,
}

impl AlertPublisher {
    pub fn new(endpoint: &str) -> Result<Self> {
        info!("Initializing ZMQ alert publisher on {}", endpoint);
        
        let context = Context::new();
        let socket = context.socket(zmq::PUB)?;
        
        // Set socket options for better performance
        socket.set_sndhwm(10000)?; // High water mark
        socket.set_linger(0)?; // Don't wait on close
        
        // Bind to endpoint
        socket.bind(endpoint)?;
        
        info!("✅ ZMQ publisher bound to {}", endpoint);
        
        Ok(Self {
            context,
            socket,
            endpoint: endpoint.to_string(),
            enabled: true,
            published_count: 0,
        })
    }
    
    pub fn publish_event(&mut self, event: &MarketEvent) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        
        // Only publish significant events
        match event.event_type {
            EventType::ScamAlert | EventType::LiquidityWarning | EventType::LargeTrade => {
                let alert = AlertMessage::from(event);
                let json = serde_json::to_string(&alert)?;
                
                // Send without blocking
                match self.socket.send(&json, zmq::DONTWAIT) {
                    Ok(_) => {
                        self.published_count += 1;
                        if self.published_count % 100 == 0 {
                            info!("Published {} alerts via ZMQ", self.published_count);
                        }
                    }
                    Err(zmq::Error::EAGAIN) => {
                        warn!("ZMQ publisher buffer full, dropping alert");
                    }
                    Err(e) => {
                        error!("Failed to publish alert: {}", e);
                    }
                }
            }
            _ => {
                // Skip minor events
            }
        }
        
        Ok(())
    }
    
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        info!("ZMQ publisher {}", if enabled { "enabled" } else { "disabled" });
    }
    
    pub fn get_stats(&self) -> PublisherStats {
        PublisherStats {
            endpoint: self.endpoint.clone(),
            enabled: self.enabled,
            published_count: self.published_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PublisherStats {
    pub endpoint: String,
    pub enabled: bool,
    pub published_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_alert_message_conversion() {
        let event = MarketEvent {
            event_type: EventType::ScamAlert,
            severity: Severity::Critical,
            tx_hash: "0x1234567890abcdef".to_string(),
            pool_address: "0xpool".to_string(),
            token_address: "0xtoken".to_string(),
            timestamp: 1234567890,
            confidence: 0.95,
            details: "Test scam alert".to_string(),
            metrics: crate::signal_engine::EventMetrics {
                eth_change: -10.0,
                token_change: 10000.0,
                eth_percent: -0.5,
                token_percent: 0.5,
                new_eth_reserve: 10.0,
                new_token_reserve: 20000.0,
                token_symbol: "TEST".to_string(),
                ..Default::default()
            },
        };
        
        let alert = AlertMessage::from(&event);
        
        assert_eq!(alert.severity, "Critical");
        assert_eq!(alert.event_type, "ScamAlert");
        assert_eq!(alert.current_eth_reserve, 20.0); // 10 - (-10)
        assert_eq!(alert.simulated_eth_reserve, 10.0);
        assert_eq!(alert.eth_change_percent, -50.0);
    }
}