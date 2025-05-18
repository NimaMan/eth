// src/bin/test_scam_detection.rs
//
// Test binary for the ScamDetectionEngine.
// This creates a test pool cache with sample data and tests the scam detection 
// logic with various transaction scenarios.

use mempool_processor::pool_subscriber::{PoolSubscriber, types::PoolUpdate, cache::PoolStateCache};
use mempool_processor::scam_detection::{ScamDetectionEngine, ScamDetectionConfig};
use tracing::{info, debug};
use eyre::Result;
use clap::Parser;
use std::collections::HashMap;
use std::sync::Arc;
use ethers::types::H256;

#[derive(Parser, Debug)]
struct Args {
    /// ETH threshold for scam detection (default: 0.05 ETH)
    #[arg(long, default_value = "0.05")]
    eth_threshold: f64,
    
    /// Percentage threshold for large withdrawals (default: 0.5 or 50%)
    #[arg(long, default_value = "0.5")]
    percentage_threshold: f64,
    
    /// Run tests in interactive mode, asking for transaction details
    #[arg(long)]
    interactive: bool,
}

// Helper to create a test pool cache with sample data
fn create_test_pool_cache(eth_threshold: f64) -> Arc<PoolStateCache> {
    let cache = PoolStateCache::new(eth_threshold);
    
    // Add some test pools
    let mut updates = HashMap::new();
    
    // Pool 1: Healthy reserves
    updates.insert(
        "0x1234567890abcdef1234567890abcdef12345678".to_string(),
        PoolUpdate {
            eth_reserve: 10.0,
            token_address: "0xtoken1".to_string(),
            block_number: 12345,
            update_time: chrono::Utc::now().timestamp() as f64,
        }
    );
    
    // Pool 2: Low reserves
    updates.insert(
        "0x2234567890abcdef1234567890abcdef12345678".to_string(),
        PoolUpdate {
            eth_reserve: 0.1,
            token_address: "0xtoken2".to_string(),
            block_number: 12345,
            update_time: chrono::Utc::now().timestamp() as f64,
        }
    );
    
    // Pool 3: Medium reserves
    updates.insert(
        "0x3234567890abcdef1234567890abcdef12345678".to_string(),
        PoolUpdate {
            eth_reserve: 2.5,
            token_address: "0xtoken3".to_string(),
            block_number: 12345,
            update_time: chrono::Utc::now().timestamp() as f64,
        }
    );
    
    cache.update_pools(updates.iter());
    Arc::new(cache)
}

fn run_test_scenarios(engine: &ScamDetectionEngine, config: &ScamDetectionConfig) {
    info!("Running test scenarios with ETH threshold: {}, percentage threshold: {}", 
          config.eth_threshold, config.percentage_threshold);
    
    // Scenario 1: Transaction that depletes a low-reserve pool
    let tx_hash = H256::random();
    info!("Scenario 1: Transaction depleting a low-reserve pool");
    debug!("Testing transaction {} from sender 0xbadhacker", tx_hash);
    
    let alerts = engine.process_transaction(
        tx_hash,
        "0xbadhacker".to_string(),
        vec![
            // Format: (pool_address, current_eth, new_eth)
            ("0x2234567890abcdef1234567890abcdef12345678".to_string(), 0.1, 0.01)
        ]
    );
    
    if alerts.is_empty() {
        info!("❌ FAILED: No alerts generated for low-reserve pool depletion");
    } else {
        info!("✅ SUCCESS: Generated {} alert(s) for low-reserve pool depletion", alerts.len());
        for (i, alert) in alerts.iter().enumerate() {
            debug!("Alert {}: {:?}", i+1, alert);
        }
    }
    
    // Scenario 2: Transaction that makes a large withdrawal from a healthy pool
    let tx_hash = H256::random();
    info!("Scenario 2: Transaction making a large withdrawal (>50%) from a healthy pool");
    debug!("Testing transaction {} from sender 0xwhale", tx_hash);
    
    let alerts = engine.process_transaction(
        tx_hash,
        "0xwhale".to_string(),
        vec![
            // Format: (pool_address, current_eth, new_eth)
            ("0x1234567890abcdef1234567890abcdef12345678".to_string(), 10.0, 2.0)
        ]
    );
    
    if alerts.is_empty() {
        info!("❌ FAILED: No alerts generated for large withdrawal");
    } else {
        info!("✅ SUCCESS: Generated {} alert(s) for large withdrawal", alerts.len());
        for (i, alert) in alerts.iter().enumerate() {
            debug!("Alert {}: {:?}", i+1, alert);
        }
    }
    
    // Scenario 3: Safe transaction that shouldn't trigger alerts
    let tx_hash = H256::random();
    info!("Scenario 3: Safe transaction with small withdrawal");
    debug!("Testing transaction {} from sender 0xnormaluser", tx_hash);
    
    let alerts = engine.process_transaction(
        tx_hash,
        "0xnormaluser".to_string(),
        vec![
            // Format: (pool_address, current_eth, new_eth)
            ("0x1234567890abcdef1234567890abcdef12345678".to_string(), 10.0, 9.0)
        ]
    );
    
    if alerts.is_empty() {
        info!("✅ SUCCESS: No alerts generated for safe transaction");
    } else {
        info!("❌ FAILED: Generated {} alert(s) for safe transaction", alerts.len());
        for (i, alert) in alerts.iter().enumerate() {
            debug!("Alert {}: {:?}", i+1, alert);
        }
    }
    
    // Scenario 4: Multiple pool effects in one transaction
    let tx_hash = H256::random();
    info!("Scenario 4: Transaction affecting multiple pools");
    debug!("Testing transaction {} from sender 0xmultipooltx", tx_hash);
    
    let alerts = engine.process_transaction(
        tx_hash,
        "0xmultipooltx".to_string(),
        vec![
            // Format: (pool_address, current_eth, new_eth)
            ("0x1234567890abcdef1234567890abcdef12345678".to_string(), 10.0, 9.5),
            ("0x2234567890abcdef1234567890abcdef12345678".to_string(), 0.1, 0.02),
            ("0x3234567890abcdef1234567890abcdef12345678".to_string(), 2.5, 0.5)
        ]
    );
    
    info!("Generated {} alert(s) for multi-pool transaction", alerts.len());
    if alerts.len() == 2 {
        info!("✅ SUCCESS: Correctly detected 2 suspicious pool changes (low reserve and large withdrawal)");
    } else {
        info!("❌ FAILED: Expected 2 alerts, got {}", alerts.len());
    }
    
    for (i, alert) in alerts.iter().enumerate() {
        debug!("Alert {}: {:?}", i+1, alert);
    }
}

fn run_interactive_mode(engine: &ScamDetectionEngine) {
    use std::io::{self, Write};
    
    info!("Interactive mode: Enter transaction details");
    
    let mut tx_count = 0;
    
    loop {
        tx_count += 1;
        println!("\nTransaction #{}", tx_count);
        println!("Enter sender address (or 'q' to quit):");
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut sender = String::new();
        io::stdin().read_line(&mut sender).unwrap();
        let sender = sender.trim();
        
        if sender == "q" || sender == "quit" || sender == "exit" {
            break;
        }
        
        // Generate a random tx hash
        let tx_hash = H256::random();
        info!("Using transaction hash: {}", tx_hash);
        
        let mut affected_pools = Vec::new();
        
        println!("\nEnter pool details (one per line, format: 'pool_address current_eth new_eth')");
        println!("Enter a blank line when done.");
        
        loop {
            print!("> ");
            io::stdout().flush().unwrap();
            
            let mut line = String::new();
            io::stdin().read_line(&mut line).unwrap();
            let line = line.trim();
            
            if line.is_empty() {
                break;
            }
            
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() != 3 {
                println!("Invalid format. Please use: 'pool_address current_eth new_eth'");
                continue;
            }
            
            let pool_address = parts[0].to_string();
            let current_eth = parts[1].parse::<f64>().unwrap_or_else(|_| {
                println!("Invalid current_eth value. Using 0.0");
                0.0
            });
            let new_eth = parts[2].parse::<f64>().unwrap_or_else(|_| {
                println!("Invalid new_eth value. Using 0.0");
                0.0
            });
            
            affected_pools.push((pool_address, current_eth, new_eth));
        }
        
        if affected_pools.is_empty() {
            println!("No pool details entered. Skipping transaction.");
            continue;
        }
        
        info!("Processing transaction with {} affected pools", affected_pools.len());
        let alerts = engine.process_transaction(tx_hash, sender.to_string(), affected_pools);
        
        println!("\nDetection Results:");
        if alerts.is_empty() {
            println!("No scam alerts generated. Transaction appears safe.");
        } else {
            println!("⚠️ ALERTS GENERATED: {} potential issues found", alerts.len());
            for (i, alert) in alerts.iter().enumerate() {
                println!("Alert {}: Pool {} would go from {} to {} ETH ({})", 
                         i+1, alert.pool_address, 
                         alert.current_eth_reserve, 
                         alert.simulated_eth_reserve,
                         match alert.reason {
                             mempool_processor::scam_detection::ScamAlertReason::EthReserveDepleted => 
                                 "ETH reserve depleted",
                             mempool_processor::scam_detection::ScamAlertReason::LargeEthWithdrawal => 
                                 "Large ETH withdrawal",
                             _ => "Other reason"
                         });
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Basic tracing setup
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            "info,mempool_processor=debug,test_scam_detection=trace"
        ))
        .init();

    // Create detection configuration
    let config = ScamDetectionConfig {
        eth_threshold: args.eth_threshold,
        percentage_threshold: args.percentage_threshold,
    };

    info!("Starting ScamDetectionEngine test with:");
    info!("  ETH threshold: {} ETH", config.eth_threshold);
    info!("  Percentage threshold: {}%", config.percentage_threshold * 100.0);

    // Create a test pool cache and scam detection engine
    let pool_cache = create_test_pool_cache(config.eth_threshold);
    let engine = ScamDetectionEngine::new(pool_cache, config.clone());
    
    if args.interactive {
        run_interactive_mode(&engine);
    } else {
        run_test_scenarios(&engine, &config);
    }
    
    info!("ScamDetectionEngine test completed.");
    Ok(())
} 