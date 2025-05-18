// src/bin/test_pool_subscriber.rs

use mempool_processor::pool_subscriber::PoolSubscriber;
use tracing::{info, error};
use eyre::Result;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    /// ETH threshold for scam detection (default: 0.05 ETH)
    #[arg(long, default_value = "0.05")]
    eth_threshold: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Basic tracing setup - can be expanded if needed
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info,mempool_processor=debug"))
        .init();

    info!("Starting Pool Subscriber test binary...");
    info!("Using ETH threshold: {} ETH", args.eth_threshold);

    // Create the pool subscriber with the specified ETH threshold
    let subscriber = PoolSubscriber::new(args.eth_threshold);

    // Run the listener. This will loop indefinitely until an error or manual stop.
    if let Err(e) = subscriber.start_listening().await {
        error!("PoolSubscriber encountered an error: {}", e);
        return Err(eyre::eyre!("Subscriber failed: {}", e));
    }
    
    // This part will likely not be reached if start_listening runs an infinite loop
    info!("Pool Subscriber test binary finished."); 
    Ok(())
} 