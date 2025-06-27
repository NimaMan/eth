//! High-performance alert receiver
//!
//! Receives execution alerts via ZMQ for ultra-low latency

use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use zmq::Context;
use std::thread;
use std::time::Duration;

use super::types::Alert;

/// Alert receiver configuration
#[derive(Debug, Clone)]
pub struct ReceiverConfig {
    /// ZMQ endpoint to connect to
    pub endpoint: String,
    /// Receive timeout in milliseconds
    pub timeout_ms: i32,
    /// Reconnect delay on connection failure
    pub reconnect_delay: Duration,
}

impl Default for ReceiverConfig {
    fn default() -> Self {
        Self {
            endpoint: "tcp://localhost:5559".to_string(),
            timeout_ms: 1000, // Reduced from 5000ms for lower latency
            reconnect_delay: Duration::from_secs(1), // Faster reconnect
        }
    }
}

/// ZMQ Alert Receiver
pub struct AlertReceiver {
    config: ReceiverConfig,
    tx: mpsc::Sender<Alert>,
}

impl AlertReceiver {
    /// Create new alert receiver
    pub fn new(config: ReceiverConfig, tx: mpsc::Sender<Alert>) -> Self {
        Self { config, tx }
    }
    
    /// Run the receiver loop
    pub async fn run(&self) {
        let config = self.config.clone();
        let tx = self.tx.clone();
        
        // Run ZMQ in dedicated thread for performance
        thread::spawn(move || {
            if let Err(e) = Self::zmq_loop(config, tx) {
                error!("ZMQ receiver error: {}", e);
            }
        });
        
        // Keep async task alive
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
    
    /// ZMQ receive loop
    fn zmq_loop(
        config: ReceiverConfig,
        tx: mpsc::Sender<Alert>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ctx = Context::new();
        
        loop {
            info!("Connecting to alert endpoint: {}", config.endpoint);
            
            let subscriber = ctx.socket(zmq::SUB)?;
            subscriber.set_rcvtimeo(config.timeout_ms)?;
            subscriber.set_linger(0)?; // Don't wait on close
            subscriber.connect(&config.endpoint)?;
            subscriber.set_subscribe(b"")?; // Subscribe to all messages
            
            info!("Connected to alert stream");
            
            loop {
                match subscriber.recv_msg(0) {
                    Ok(msg) => {
                        let data = msg.as_str().unwrap_or("");
                        debug!("Received alert data: {} bytes", data.len());
                        
                        match serde_json::from_str::<Alert>(data) {
                            Ok(alert) => {
                                info!("Alert received: {} for token {}", 
                                    alert.id, alert.token_address);
                                
                                // Send alert to executor
                                if let Err(e) = tx.blocking_send(alert) {
                                    error!("Failed to send alert: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse alert: {}", e);
                                debug!("Raw data: {}", data);
                            }
                        }
                    }
                    Err(e) => {
                        if e == zmq::Error::EAGAIN {
                            // Timeout - normal behavior
                            debug!("No alerts received (timeout)");
                        } else {
                            error!("Receive error: {}", e);
                            break; // Reconnect
                        }
                    }
                }
            }
            
            warn!("Disconnected, reconnecting in {:?}", config.reconnect_delay);
            thread::sleep(config.reconnect_delay);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert_processor::{Action, ExecutionParams, Priority};
    
    #[test]
    fn test_alert_parsing() {
        let json = r#"{
            "id": "test-123",
            "timestamp": 1234567890,
            "token_address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "pool_address": "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",
            "action": "Sell",
            "params": {
                "amount": "1000000000",
                "slippage": 0.05,
                "max_gas_price": null,
                "deadline_seconds": 300,
                "priority": "High"
            }
        }"#;
        
        let alert: Alert = serde_json::from_str(json).unwrap();
        assert_eq!(alert.id, "test-123");
        assert_eq!(alert.timestamp, 1234567890);
    }
    
    #[tokio::test]
    async fn test_receiver_creation() {
        let (tx, _rx) = mpsc::channel(10);
        let config = ReceiverConfig::default();
        let receiver = AlertReceiver::new(config, tx);
        
        // Verify receiver was created
        assert_eq!(receiver.config.timeout_ms, 1000);
    }
}