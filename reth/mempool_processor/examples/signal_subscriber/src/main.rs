//! ZMQ Subscriber for Mempool Signal Detector
//! 
//! This example demonstrates how to subscribe to real-time signals from the
//! mempool signal detector using Rust.

use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignalAlert {
    pub alert_type: String,
    pub function_name: String,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub value: String,
    pub gas_price: String,
    pub selector: String,
    pub timestamp: String,
    pub detection_latency_us: u64,
}

struct SignalSubscriber {
    endpoint: String,
    stats_liquidity: Arc<AtomicU64>,
    stats_trading: Arc<AtomicU64>,
    stats_total: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
}

impl SignalSubscriber {
    fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            stats_liquidity: Arc::new(AtomicU64::new(0)),
            stats_trading: Arc::new(AtomicU64::new(0)),
            stats_total: Arc::new(AtomicU64::new(0)),
            running: Arc::new(AtomicBool::new(true)),
        }
    }
    
    fn handle_signal(&self, signal: SignalAlert) {
        let total = self.stats_total.fetch_add(1, Ordering::Relaxed) + 1;
        
        // Update specific counter
        match signal.alert_type.as_str() {
            "liquidity_removal" => {
                self.stats_liquidity.fetch_add(1, Ordering::Relaxed);
            }
            "trading_enabled" => {
                self.stats_trading.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
        
        // Display signal
        info!("📨 SIGNAL RECEIVED! #{}", total);
        info!("  Type: {}", signal.alert_type.to_uppercase());
        info!("  Function: {}", signal.function_name);
        info!("  TX Hash: {}", signal.tx_hash);
        info!("  From: {}", signal.from_address);
        info!("  To: {}", signal.to_address);
        info!("  Value: {}", signal.value);
        info!("  Gas Price: {}", signal.gas_price);
        info!("  Selector: {}", signal.selector);
        info!("  Timestamp: {}", signal.timestamp);
        info!("{}", "-".repeat(80));
    }
    
    fn print_stats(&self) {
        info!("\n=== STATISTICS ===");
        info!("Total signals received: {}", self.stats_total.load(Ordering::Relaxed));
        info!("Liquidity removals: {}", self.stats_liquidity.load(Ordering::Relaxed));
        info!("Trading enabled: {}", self.stats_trading.load(Ordering::Relaxed));
    }
    
    fn run(&self) {
        info!("🔌 Connecting to ZMQ publisher at {}", self.endpoint);
        
        // Create ZMQ context and subscriber
        let context = zmq::Context::new();
        let subscriber = context.socket(zmq::SUB).expect("Failed to create socket");
        
        // Connect and subscribe to all messages
        subscriber.connect(&self.endpoint).expect("Failed to connect");
        subscriber.set_subscribe(b"").expect("Failed to subscribe");
        
        // Set receive timeout
        subscriber.set_rcvtimeo(1000).expect("Failed to set timeout");
        
        info!("✅ Connected! Waiting for signals...\n");
        
        // Setup Ctrl+C handler
        let running = self.running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::Relaxed);
        }).expect("Error setting Ctrl-C handler");
        
        // Main loop
        while self.running.load(Ordering::Relaxed) {
            match subscriber.recv_string(0) {
                Ok(Ok(message)) => {
                    // Parse JSON
                    match serde_json::from_str::<SignalAlert>(&message) {
                        Ok(signal) => {
                            self.handle_signal(signal);
                        }
                        Err(e) => {
                            error!("Failed to parse JSON: {}", e);
                            error!("Raw message: {}", message);
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!("Invalid UTF-8 in message: {}", e);
                }
                Err(zmq::Error::EAGAIN) => {
                    // Timeout - no message, continue
                    continue;
                }
                Err(e) => {
                    error!("ZMQ Error: {}", e);
                    break;
                }
            }
        }
        
        info!("\n🛑 Shutting down...");
        self.print_stats();
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    println!("{}", "=".repeat(80));
    println!("Mempool Signal Detector - ZMQ Subscriber Example (Rust)");
    println!("{}", "=".repeat(80));
    
    // Create and run subscriber
    let subscriber = SignalSubscriber::new("tcp://127.0.0.1:5556");
    subscriber.run();
}