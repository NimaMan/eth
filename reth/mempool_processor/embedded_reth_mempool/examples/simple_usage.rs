use embedded_reth_mempool::{EmbeddedRethConfig, EmbeddedRethListener};
use std::time::Duration;
use tokio::time::timeout;
use tracing::info;

/// Simple usage example for embedded Reth mempool detection
/// 
/// Run with: cargo run --example simple_usage
#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("embedded_reth_mempool=info,simple_usage=info")
        .init();

    info!("Starting simple embedded Reth example...");

    // Create listener with default config
    let config = EmbeddedRethConfig::default();
    let listener = EmbeddedRethListener::new(config).await?;

    // Start processing
    listener.start_processing().await?;

    // Subscribe to transactions
    let mut tx_stream = listener.subscribe();

    info!("Listening for transactions (30 seconds)...");

    // Process transactions for 30 seconds
    let duration = Duration::from_secs(30);
    let start = std::time::Instant::now();

    while start.elapsed() < duration {
        match timeout(Duration::from_secs(1), tx_stream.next()).await {
            Ok(Some(tx)) => {
                info!(
                    "Transaction: {} | Gas: {} | Latency: {}μs | Contract: {}",
                    tx.hash_short(),
                    tx.gas_limit(),
                    tx.latency_us(),
                    tx.is_contract_interaction()
                );
            }
            Ok(None) => {
                info!("Transaction stream ended");
                break;
            }
            Err(_) => {
                // Timeout - no transactions in 1 second
                continue;
            }
        }
    }

    // Print final metrics
    let report = listener.metrics().report();
    println!("\n{}", report);

    // Shutdown
    listener.shutdown().await?;
    info!("Shutdown complete");

    Ok(())
}