//! Multipart Subscriber Example
//! 
//! Algorithm:
//! 1. Connect to unified publisher on port 5560
//! 2. Subscribe to specific topics or all topics
//! 3. Receive multipart messages [topic, json_data]
//! 4. Parse UnifiedSignal from JSON
//! 5. Display signal details based on type
//! 6. Track statistics by signal type

use mempool_processor::publishers::{UnifiedSignal, SignalData};
use zmq;
use serde_json;
use tracing::{info, warn, error};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

struct MultipartSubscriber {
    endpoint: String,
    topics: Vec<String>,
    stats: HashMap<String, AtomicU64>,
    running: Arc<AtomicBool>,
}

impl MultipartSubscriber {
    fn new(endpoint: &str, topics: Vec<String>) -> Self {
        let mut stats = HashMap::new();
        stats.insert("total".to_string(), AtomicU64::new(0));
        stats.insert("tax_manipulation".to_string(), AtomicU64::new(0));
        stats.insert("liquidity_removal".to_string(), AtomicU64::new(0));
        stats.insert("pool_drain".to_string(), AtomicU64::new(0));
        stats.insert("trading_enabled".to_string(), AtomicU64::new(0));
        
        Self {
            endpoint: endpoint.to_string(),
            topics,
            stats,
            running: Arc::new(AtomicBool::new(true)),
        }
    }
    
    fn handle_signal(&self, topic: &str, signal: UnifiedSignal) {
        // Update statistics
        self.stats.get("total").unwrap().fetch_add(1, Ordering::Relaxed);
        if let Some(counter) = self.stats.get(topic) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
        
        // Display signal header
        info!("📨 SIGNAL RECEIVED!");
        info!("  Topic: {}", topic);
        info!("  Type: {:?}", signal.base.signal_type);
        info!("  Severity: {:?}", signal.base.severity);
        info!("  Confidence: {:.2}", signal.base.confidence);
        info!("  TX: {}", signal.base.tx_hash);
        info!("  Token: {}", signal.base.token_address);
        
        // Display type-specific data
        match &signal.data {
            SignalData::TaxManipulation(tax_data) => {
                info!("  🚨 TAX MANIPULATION DETECTED!");
                info!("    Pattern: {}", tax_data.pattern);
                info!("    Buy tax: {}% → {}%", tax_data.current_buy_tax, tax_data.predicted_buy_tax);
                info!("    Sell tax: {}% → {}%", tax_data.current_sell_tax, tax_data.predicted_sell_tax);
                info!("    Manipulator: {}", tax_data.manipulator_address);
            }
            SignalData::LiquidityRemoval(liq_data) => {
                info!("  💧 LIQUIDITY REMOVAL!");
                info!("    Function: {}", liq_data.function_name);
                if let Some(percent) = liq_data.percentage {
                    info!("    Percentage: {:.1}%", percent);
                }
                if let Some(eth) = liq_data.eth_amount {
                    info!("    ETH amount: {:.2}", eth);
                }
            }
            SignalData::PoolDrain(drain_data) => {
                info!("  🚨🚨 POOL DRAIN ALERT! 🚨🚨");
                info!("    ETH drained: {:.2} ({:.1}%)", 
                     drain_data.eth_drained, 
                     drain_data.drain_percentage);
                info!("    Remaining: {:.2} ETH", drain_data.new_eth_reserve);
                if drain_data.is_complete_drain {
                    info!("    ⚠️  COMPLETE DRAIN DETECTED!");
                }
            }
            SignalData::TradingStatus(trade_data) => {
                info!("  📊 TRADING STATUS CHANGE!");
                info!("    Function: {}", trade_data.function_name);
                info!("    Enabled: {}", trade_data.enabled);
                if let Some(eth) = trade_data.initial_liquidity_eth {
                    info!("    Initial liquidity: {:.2} ETH", eth);
                }
            }
            _ => {
                info!("  Details: {}", signal.details);
            }
        }
        
        info!("{}", "-".repeat(80));
    }
    
    fn print_stats(&self) {
        info!("\n=== STATISTICS ===");
        info!("Total signals: {}", self.stats.get("total").unwrap().load(Ordering::Relaxed));
        info!("By type:");
        for (topic, counter) in &self.stats {
            if topic != "total" {
                let count = counter.load(Ordering::Relaxed);
                if count > 0 {
                    info!("  - {}: {}", topic, count);
                }
            }
        }
    }
    
    fn run(&self) {
        info!("🔌 Connecting to unified publisher at {}", self.endpoint);
        
        // Create ZMQ context and subscriber
        let context = zmq::Context::new();
        let subscriber = context.socket(zmq::SUB).expect("Failed to create socket");
        
        // Connect to publisher
        subscriber.connect(&self.endpoint).expect("Failed to connect");
        
        // Subscribe to topics
        if self.topics.is_empty() {
            // Subscribe to all
            subscriber.set_subscribe(b"").expect("Failed to subscribe");
            info!("📡 Subscribed to ALL topics");
        } else {
            // Subscribe to specific topics
            for topic in &self.topics {
                subscriber.set_subscribe(topic.as_bytes()).expect("Failed to subscribe");
                info!("📡 Subscribed to topic: {}", topic);
            }
        }
        
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
            // Receive multipart message
            match subscriber.recv_multipart(0) {
                Ok(parts) => {
                    if parts.len() != 2 {
                        warn!("Invalid multipart message: expected 2 parts, got {}", parts.len());
                        continue;
                    }
                    
                    // Parse topic and data
                    let topic = match String::from_utf8(parts[0].clone()) {
                        Ok(t) => t,
                        Err(e) => {
                            error!("Invalid UTF-8 in topic: {}", e);
                            continue;
                        }
                    };
                    
                    let json_data = match String::from_utf8(parts[1].clone()) {
                        Ok(d) => d,
                        Err(e) => {
                            error!("Invalid UTF-8 in data: {}", e);
                            continue;
                        }
                    };
                    
                    // Parse UnifiedSignal
                    match serde_json::from_str::<UnifiedSignal>(&json_data) {
                        Ok(signal) => {
                            self.handle_signal(&topic, signal);
                        }
                        Err(e) => {
                            error!("Failed to parse signal JSON: {}", e);
                            error!("Topic: {}", topic);
                            error!("Raw JSON: {}", json_data);
                        }
                    }
                }
                Err(zmq::Error::EAGAIN) => {
                    // Timeout - no message
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
    println!("Unified Publisher - Multipart Subscriber Example");
    println!("{}", "=".repeat(80));
    println!();
    
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let topics = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        vec![] // Subscribe to all
    };
    
    if !topics.is_empty() {
        println!("Subscribing to specific topics: {:?}", topics);
    } else {
        println!("Subscribing to ALL topics");
    }
    println!();
    
    // Create and run subscriber
    let subscriber = MultipartSubscriber::new("tcp://127.0.0.1:5560", topics);
    subscriber.run();
}