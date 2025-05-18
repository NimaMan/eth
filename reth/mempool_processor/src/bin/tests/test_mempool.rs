/*
 * Ethereum Mempool Processor Tester
 * 
 * This tool tests the modular mempool processor by:
 * 1. Creating mock transaction data that simulates real mempool activity
 * 2. Processing transactions through the filtering system
 * 3. Generating alerts for interesting transactions
 * 4. Publishing alerts via ZMQ (can be consumed by the Python subscriber)
 */

use mempool_processor::mempool_processor::*;
use ethers::prelude::*;
use eyre::Result;
use std::time::{Duration, Instant};
use tracing::{info, debug, error};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    /// Number of test transactions to generate
    #[arg(long, default_value = "50")]
    tx_count: usize,
    
    /// Number of high-value transactions to include
    #[arg(long, default_value = "5")]
    high_value_count: usize,
    
    /// Addresses to watch (comma separated)
    #[arg(long, default_value = "")]
    watched_addresses: String,
    
    /// Value threshold in ETH (0.1 ETH default)
    #[arg(long, default_value = "0.1")]
    threshold_eth: f64,
    
    /// Whether to publish to ZMQ
    #[arg(long)]
    publish: bool,
    
    /// Verbose logging mode
    #[arg(short, long)]
    verbose: bool,
}

fn format_eth(wei: U256) -> String {
    let eth_value = wei.as_u128() as f64 / 1_000_000_000_000_000_000f64;
    format!("{:.6} ETH", eth_value)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Configure tracing based on verbosity
    let env_filter = if args.verbose {
        "mempool_processor=debug,info"
    } else {
        "mempool_processor=info"
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(env_filter))
        .init();

    info!("======== Ethereum Mempool Processor Tester ========");
    info!("Generating {} test transactions ({} high-value)", args.tx_count, args.high_value_count);
    
    // Parse watched addresses
    let watched = if args.watched_addresses.is_empty() {
        Vec::new()
    } else {
        args.watched_addresses.split(',').map(|s| s.to_string()).collect()
    };
    
    if !watched.is_empty() {
        info!("Watching {} addresses", watched.len());
    }
    
    // Create threshold from ETH value
    let wei_threshold = U256::from((args.threshold_eth * 1e18) as u64);
    info!("Value threshold: {} ({})", wei_threshold, format_eth(wei_threshold));
    
    // Set up transaction processor
    let filter = TransactionFilter {
        min_value: wei_threshold,
        watched_addresses: watched.into_iter().collect(),
        min_gas_price: Some(U256::from(50_000_000_000u64)), // 50 gwei
    };
    
    let processor = TransactionProcessor::new(filter);
    
    // Create mock transaction source
    let mut mock_source = MockTransactionSource::new();
    
    // Generate normal-value transactions
    generate_random_transactions(&mut mock_source, args.tx_count - args.high_value_count);
    
    // Generate high-value transactions
    generate_high_value_transactions(&mut mock_source, args.high_value_count);
    
    // Get transactions
    let transactions = mock_source.get_transactions().await?;
    info!("Generated {} total transactions", transactions.len());
    
    // Process transactions
    let start_time = Instant::now();
    let alerts = processor.process_transactions(transactions);
    let process_time = start_time.elapsed();
    
    info!("Processing complete in {:?}", process_time);
    info!("Generated {} alerts", alerts.len());
    
    // Print alert details
    for (i, alert) in alerts.iter().enumerate() {
        let from_hex = hex::encode(&alert.transaction.from);
        let to_hex = if let Some(to) = &alert.transaction.to {
            hex::encode(to)
        } else {
            "contract creation".to_string()
        };
        
        info!("Alert #{}: {:?}", i+1, alert.reason);
        info!("  From: 0x{}", from_hex);
        info!("  To:   0x{}", to_hex);
        info!("  Value: {} ({})", alert.transaction.value, format_eth(alert.transaction.value));
        
        if let Some(gas_price) = alert.transaction.gas_price {
            let gwei = gas_price.as_u128() as f64 / 1_000_000_000f64;
            info!("  Gas Price: {:.2} gwei", gwei);
        }
        info!("------------------------");
    }
    
    // Publish to ZMQ if enabled
    if args.publish && !alerts.is_empty() {
        info!("Publishing alerts to ZeroMQ...");
        
        // Set up ZMQ publisher
        let zmq_ctx = zmq::Context::new();
        let publisher = ZmqAlertPublisher::new(&zmq_ctx, "ipc:///tmp/mempool_feed")?;
        
        // Track successful publishes
        let published = Arc::new(AtomicUsize::new(0));
        let published_clone = published.clone();
        
        // Process alerts
        for alert in &alerts {
            match publisher.publish_alert(alert) {
                Ok(_) => {
                    published_clone.fetch_add(1, Ordering::SeqCst);
                },
                Err(e) => {
                    error!("Failed to publish alert: {}", e);
                }
            }
            
            // Small delay between alerts
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        info!("Published {} alerts via ZeroMQ", published.load(Ordering::SeqCst));
        info!("You can run the Python subscriber to view these alerts");
    }
    
    Ok(())
}

fn generate_random_transactions(source: &mut MockTransactionSource, count: usize) {
    // Some example addresses (can be any valid Ethereum addresses)
    let from_addresses = [
        "0x5aeda56215b626dad1c48c6390b205736e614c91",
        "0x6b4712ae9797f71847da984efdc5d4e3c592c477",
        "0x8e4d8535a9c19ac566ff3d4abbbe95eb10e22b89",
        "0x923cb9c11b0f78c3a9c43d7eb478dba7f73a6ea9",
    ];
    
    let to_addresses = [
        "0xa49830dc0eb7e08c3a953f6f26024a72f8e3a32a",
        "0xb54fc2f2e2c70e87b3ae414cb3ab963b2a0fe698",
        "0xc3fb417aab0bddedac48abd0accf4ceb1615ef36",
        "0xd98b10ea846a29228d1c8afd5a324805c41b2fc4",
    ];
    
    for _ in 0..count {
        let from_idx = fastrand::usize(..from_addresses.len());
        let to_idx = fastrand::usize(..to_addresses.len());
        
        // Generate random value (0.001 to 0.05 ETH)
        let value_eth = fastrand::f64() * 0.049 + 0.001;
        
        source.add_high_value_tx(
            from_addresses[from_idx],
            to_addresses[to_idx],
            value_eth
        );
    }
}

fn generate_high_value_transactions(source: &mut MockTransactionSource, count: usize) {
    // Some example addresses (can be any valid Ethereum addresses)
    let from_addresses = [
        "0x1234567890123456789012345678901234567890",
        "0xabcdef0123456789abcdef0123456789abcdef01",
        "0x9876543210fedcba9876543210fedcba98765432",
    ];
    
    let to_addresses = [
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "0xcccccccccccccccccccccccccccccccccccccccc",
    ];
    
    for _ in 0..count {
        let from_idx = fastrand::usize(..from_addresses.len());
        let to_idx = fastrand::usize(..to_addresses.len());
        
        // Generate random high value (0.5 to 5 ETH)
        let value_eth = fastrand::f64() * 4.5 + 0.5;
        
        source.add_high_value_tx(
            from_addresses[from_idx],
            to_addresses[to_idx],
            value_eth
        );
    }
} 