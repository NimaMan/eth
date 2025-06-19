//! Mock Alert Publisher for Integration Testing
//!
//! Publishes test alerts via ZMQ to validate the full alert flow

use zmq::{Context, Socket};
use serde_json::json;
use std::time::{Duration, Instant};
use std::thread;

pub struct MockAlertPublisher {
    socket: Socket,
    alerts_sent: usize,
}

impl MockAlertPublisher {
    pub fn new(endpoint: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let context = Context::new();
        let socket = context.socket(zmq::PUB)?;
        socket.bind(endpoint)?;
        
        // Give socket time to bind
        thread::sleep(Duration::from_millis(100));
        
        Ok(Self {
            socket,
            alerts_sent: 0,
        })
    }
    
    pub fn send_alert(
        &mut self,
        severity: &str,
        eth_change_percent: f64,
        confidence: f64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let alert_id = format!("test_alert_{}", self.alerts_sent);
        
        let alert = json!({
            "alert_id": alert_id,
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "severity": severity,
            "event_type": "ScamAlert",
            "tx_hash": format!("0x{}", "1".repeat(64)),
            "detected_latency_us": 5000,
            "pool_address": format!("0x{}", "2".repeat(40)),
            "pool_version": "V2",
            "token_address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
            "token_symbol": "USDC",
            "token_decimals": 6,
            "current_eth_reserve": 100.0,
            "simulated_eth_reserve": 100.0 * (1.0 + eth_change_percent / 100.0),
            "eth_change_amount": 100.0 * eth_change_percent / 100.0,
            "eth_change_percent": eth_change_percent,
            "current_price": 1.0,
            "simulated_price": 0.1,
            "price_impact_percent": -90.0,
            "confidence_score": confidence,
            "gas_price_gwei": 30.0,
            "details": format!("Test alert: {} drain", eth_change_percent)
        });
        
        let message = serde_json::to_string(&alert)?;
        self.socket.send(&message, 0)?;
        self.alerts_sent += 1;
        
        Ok(alert_id.clone())
    }
    
    pub fn send_burst(&mut self, count: usize) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut alert_ids = Vec::new();
        let start = Instant::now();
        
        for i in 0..count {
            // Vary the alerts for realistic testing
            let severity = match i % 3 {
                0 => "Critical",
                1 => "High",
                _ => "Medium",
            };
            
            let drain = match i % 3 {
                0 => -85.0,
                1 => -55.0,
                _ => -30.0,
            };
            
            let alert_id = self.send_alert(severity, drain, 0.9)?;
            alert_ids.push(alert_id);
        }
        
        let elapsed = start.elapsed();
        println!("Sent {} alerts in {:?} ({:.2} alerts/sec)", 
            count, 
            elapsed,
            count as f64 / elapsed.as_secs_f64()
        );
        
        Ok(alert_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    
    #[test]
    fn test_alert_publisher_creation() {
        let publisher = MockAlertPublisher::new("tcp://127.0.0.1:15559");
        assert!(publisher.is_ok());
    }
    
    #[test]
    fn test_alert_sending() {
        let mut publisher = MockAlertPublisher::new("tcp://127.0.0.1:15560").unwrap();
        
        // Set up subscriber to verify
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();
        
        thread::spawn(move || {
            let context = Context::new();
            let subscriber = context.socket(zmq::SUB).unwrap();
            subscriber.connect("tcp://127.0.0.1:15560").unwrap();
            subscriber.set_subscribe(b"").unwrap();
            
            // Receive one message
            if let Ok(msg) = subscriber.recv_string(0) {
                if let Ok(msg) = msg {
                    received_clone.lock().unwrap().push(msg);
                }
            }
        });
        
        // Give subscriber time to connect
        thread::sleep(Duration::from_millis(100));
        
        // Send alert
        let alert_id = publisher.send_alert("Critical", -95.0, 0.99).unwrap();
        assert!(alert_id.starts_with("test_alert_"));
        
        // Give time for message to be received
        thread::sleep(Duration::from_millis(100));
        
        // Verify received
        let messages = received.lock().unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("Critical"));
        assert!(messages[0].contains("-95"));
    }
    
    #[test]
    fn test_burst_sending() {
        let mut publisher = MockAlertPublisher::new("tcp://127.0.0.1:15561").unwrap();
        
        let alert_ids = publisher.send_burst(10).unwrap();
        assert_eq!(alert_ids.len(), 10);
        
        // Verify unique IDs
        let unique_ids: std::collections::HashSet<_> = alert_ids.iter().collect();
        assert_eq!(unique_ids.len(), 10);
    }
}