//! High-performance alert receiver
//!
//! Receives execution alerts via ZMQ for ultra-low latency

use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use zmq::Context;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use super::types::Alert;
use super::validation::{AlertValidator, SignedMessage};

/// Alert receiver configuration
#[derive(Debug, Clone)]
pub struct ReceiverConfig {
    /// ZMQ endpoint to connect to
    pub endpoint: String,
    /// Receive timeout in milliseconds
    pub timeout_ms: i32,
    /// Reconnect delay on connection failure
    pub reconnect_delay: Duration,
    /// HMAC shared secret for validation
    pub hmac_secret: String,
    /// Database URL for replay protection
    pub database_url: String,
}

impl Default for ReceiverConfig {
    fn default() -> Self {
        Self {
            endpoint: "tcp://localhost:5559".to_string(),
            timeout_ms: 1000, // Reduced from 5000ms for lower latency
            reconnect_delay: Duration::from_secs(1), // Faster reconnect
            hmac_secret: std::env::var("KARTAL_HMAC_SECRET").unwrap_or_else(|_| "default-dev-secret".to_string()),
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgresql://localhost/eth_db".to_string()),
        }
    }
}

/// ZMQ Alert Receiver
pub struct AlertReceiver {
    config: ReceiverConfig,
    tx: mpsc::Sender<Alert>,
    shutdown: Arc<AtomicBool>,
}

impl AlertReceiver {
    /// Create new alert receiver
    pub fn new(config: ReceiverConfig, tx: mpsc::Sender<Alert>) -> Self {
        Self { 
            config, 
            tx,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// Run the receiver loop
    pub async fn run(&self) {
        let config = self.config.clone();
        let tx = self.tx.clone();
        let shutdown = self.shutdown.clone();
        
        // Run ZMQ in dedicated thread for performance
        thread::spawn(move || {
            if let Err(e) = Self::zmq_loop(config, tx, shutdown) {
                error!("ZMQ receiver error: {}", e);
            }
        });
        
        // Keep async task alive until shutdown
        while !self.shutdown.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        
        info!("Alert receiver shutting down");
    }
    
    /// ZMQ receive loop
    fn zmq_loop(
        config: ReceiverConfig,
        tx: mpsc::Sender<Alert>,
        shutdown: Arc<AtomicBool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ctx = Context::new();
        
        while !shutdown.load(Ordering::Relaxed) {
            info!("Connecting to alert endpoint: {}", config.endpoint);
            
            let subscriber = ctx.socket(zmq::SUB)?;
            subscriber.set_rcvtimeo(config.timeout_ms)?;
            subscriber.set_linger(0)?; // Don't wait on close
            subscriber.connect(&config.endpoint)?;
            subscriber.set_subscribe(b"")?; // Subscribe to all messages
            
            info!("Connected to alert stream");
            
            while !shutdown.load(Ordering::Relaxed) {
                match subscriber.recv_msg(0) {
                    Ok(msg) => {
                        let data = msg.as_str().unwrap_or("");
                        debug!("Received alert data: {} bytes", data.len());
                        
                        // Parse signed message
                        match SignedMessage::from_zmq_message(data) {
                            Ok(signed_msg) => {
                                // Validate HMAC signature
                                let validator = AlertValidator::new(&config.hmac_secret);
                                match validator.validate_alert(&signed_msg.payload, &signed_msg.signature) {
                                    Ok(alert) => {
                                info!("Alert received: {} for token {}", 
                                    alert.id, alert.token_address);
                                
                                // Send alert to executor
                                match tx.try_send(alert) {
                                    Ok(_) => debug!("Alert sent successfully"),
                                    Err(mpsc::error::TrySendError::Full(_)) => {
                                        warn!("Alert channel full, dropping alert");
                                    }
                                    Err(mpsc::error::TrySendError::Closed(_)) => {
                                        error!("Alert channel closed, shutting down");
                                        shutdown.store(true, Ordering::Relaxed);
                                        break;
                                    }
                                }
                                    }
                                    Err(e) => {
                                        warn!("Alert validation failed: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse signed message: {}", e);
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
        
        info!("ZMQ receiver loop terminated");
        Ok(())
    }
    
    /// Signal shutdown
    pub fn shutdown(&self) {
        info!("Shutdown requested for alert receiver");
        self.shutdown.store(true, Ordering::Relaxed);
    }
    
    /// Check if shutdown was requested
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
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