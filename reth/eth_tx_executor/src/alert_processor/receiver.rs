//! ZMQ Alert Receiver
//!
//! Connects to mempool processor and receives scam alerts in real-time

use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use zmq::Context;
use std::thread;
use std::time::Duration;

use super::types::{AlertMessage, ScamAlert};

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
            timeout_ms: 5000,
            reconnect_delay: Duration::from_secs(5),
        }
    }
}

/// ZMQ Alert Receiver
pub struct AlertReceiver {
    config: ReceiverConfig,
    tx: mpsc::Sender<ScamAlert>,
    running: bool,
}

impl AlertReceiver {
    /// Create new alert receiver
    pub fn new(config: ReceiverConfig, tx: mpsc::Sender<ScamAlert>) -> Self {
        Self {
            config,
            tx,
            running: false,
        }
    }
    
    /// Start receiving alerts in a dedicated thread
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.running {
            return Err("Receiver already running".into());
        }
        
        self.running = true;
        let config = self.config.clone();
        let tx = self.tx.clone();
        let running = self.running;
        
        // Spawn dedicated thread for ZMQ (it doesn't play well with tokio)
        thread::spawn(move || {
            if let Err(e) = Self::receiver_loop(config, tx, running) {
                error!("Alert receiver error: {}", e);
            }
        });
        
        info!("Alert receiver started on {}", self.config.endpoint);
        Ok(())
    }
    
    /// Stop the receiver
    pub fn stop(&mut self) {
        self.running = false;
        info!("Alert receiver stopping");
    }
    
    /// Main receiver loop (runs in dedicated thread)
    fn receiver_loop(
        config: ReceiverConfig,
        tx: mpsc::Sender<ScamAlert>,
        mut running: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut attempts = 0;
        
        while running {
            match Self::connect_and_receive(&config, &tx, &mut running) {
                Ok(_) => {
                    info!("Alert receiver disconnected gracefully");
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    error!("Alert receiver error (attempt {}): {}", attempts, e);
                    
                    if running {
                        info!("Reconnecting in {:?}...", config.reconnect_delay);
                        thread::sleep(config.reconnect_delay);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Connect to ZMQ and receive messages
    fn connect_and_receive(
        config: &ReceiverConfig,
        tx: &mpsc::Sender<ScamAlert>,
        running: &mut bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create ZMQ context and socket
        let context = Context::new();
        let socket = context.socket(zmq::SUB)?;
        
        // Configure socket
        socket.set_rcvtimeo(config.timeout_ms)?;
        socket.set_linger(0)?; // Don't wait on close
        
        // Connect and subscribe to all messages
        info!("Connecting to {}", config.endpoint);
        socket.connect(&config.endpoint)?;
        socket.set_subscribe(b"")?;
        
        info!("✅ Connected to alert publisher");
        
        let mut alerts_received = 0;
        let mut last_alert_time = std::time::Instant::now();
        
        // Receive loop
        while *running {
            match socket.recv_string(0) {
                Ok(Ok(message)) => {
                    alerts_received += 1;
                    let gap = last_alert_time.elapsed();
                    last_alert_time = std::time::Instant::now();
                    
                    debug!("Received alert {} (gap: {:?})", alerts_received, gap);
                    
                    // Parse and process alert
                    match serde_json::from_str::<AlertMessage>(&message) {
                        Ok(alert_msg) => {
                            match ScamAlert::from_alert_message(alert_msg) {
                                Ok(alert) => {
                                    let is_emergency = alert.is_emergency();
                                    let drain_amount = alert.eth_drain_amount();
                                    
                                    // Log based on severity
                                    if is_emergency {
                                        error!("🚨 EMERGENCY SCAM: {} - {:.2} ETH drain ({:.1}%)", 
                                            alert.token_symbol, drain_amount, alert.eth_change_percent);
                                    } else if alert.requires_action {
                                        warn!("⚠️  SCAM DETECTED: {} - {:.2} ETH drain ({:.1}%)", 
                                            alert.token_symbol, drain_amount, alert.eth_change_percent);
                                    } else {
                                        info!("ℹ️  Market event: {} - {:.1}% change", 
                                            alert.token_symbol, alert.eth_change_percent);
                                    }
                                    
                                    // Send to strategy engine
                                    if let Err(e) = tx.blocking_send(alert) {
                                        error!("Failed to send alert to strategy engine: {}", e);
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to parse alert: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to deserialize alert message: {}", e);
                            debug!("Raw message: {}", message);
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!("Invalid UTF-8 in message: {:?}", e);
                }
                Err(zmq::Error::EAGAIN) => {
                    // Timeout - normal, check if we should continue
                    debug!("No alerts for {} seconds", config.timeout_ms / 1000);
                }
                Err(e) => {
                    error!("ZMQ receive error: {}", e);
                    return Err(Box::new(e));
                }
            }
        }
        
        info!("Alert receiver shutting down (received {} alerts)", alerts_received);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_receiver_config_default() {
        let config = ReceiverConfig::default();
        assert_eq!(config.endpoint, "tcp://localhost:5559");
        assert_eq!(config.timeout_ms, 5000);
    }
    
    #[tokio::test]
    async fn test_alert_parsing() {
        let alert_json = r#"{
            "alert_id": "test_123",
            "timestamp": 1234567890,
            "severity": "Critical",
            "event_type": "ScamAlert",
            "tx_hash": "0x1234567890123456789012345678901234567890123456789012345678901234",
            "detected_latency_us": 1000,
            "pool_address": "0x1234567890123456789012345678901234567890",
            "pool_version": "V2",
            "token_address": "0x0987654321098765432109876543210987654321",
            "token_symbol": "SCAM",
            "token_decimals": 18,
            "current_eth_reserve": 100.0,
            "simulated_eth_reserve": 5.0,
            "eth_change_amount": -95.0,
            "eth_change_percent": -95.0,
            "current_price": 1000.0,
            "simulated_price": 100.0,
            "price_impact_percent": -90.0,
            "confidence_score": 0.95,
            "gas_price_gwei": 30.0,
            "details": "Critical drain detected"
        }"#;
        
        let alert_msg: AlertMessage = serde_json::from_str(alert_json).unwrap();
        let scam_alert = ScamAlert::from_alert_message(alert_msg).unwrap();
        
        assert_eq!(scam_alert.token_symbol, "SCAM");
        assert_eq!(scam_alert.eth_change_percent, -95.0);
        assert!(scam_alert.is_emergency());
        assert!(scam_alert.requires_action);
        assert_eq!(scam_alert.eth_drain_amount(), 95.0);
    }
}