/// Simple Fetcher Test
///
/// Minimal test to verify the mempool fetcher is working correctly.
/// This connects to IPC, fetches a few transactions, and exits.
///
/// Usage: cargo run --example test_fetcher_simple
use mempool_processor::mempool_fetcher::MempoolFetcherIPCClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple console logging
    tracing_subscriber::fmt().with_target(false).init();

    println!("\n🧪 Simple Mempool Fetcher Test\n");
    println!("================================\n");

    // 1. Create IPC client
    let ipc_path = mempool_processor::config::reth_ipc_path_from_env();
    println!("📡 Connecting to IPC at {}...", ipc_path);
    let ipc_client = MempoolFetcherIPCClient::new(Some(&ipc_path))?;

    // 2. Start monitoring
    println!("🚀 Starting mempool monitoring...");
    ipc_client.start().await?;

    // Give it a moment to connect
    tokio::time::sleep(Duration::from_millis(500)).await;

    println!("✅ Connected! Waiting for transactions...\n");

    // 3. Fetch some transactions
    let target_count = 10;
    let mut total_fetched = 0;
    let mut attempts = 0;
    let max_attempts = 100; // Try for ~10 seconds

    while total_fetched < target_count && attempts < max_attempts {
        let txs = ipc_client.get_transactions_instant(5).await;

        if !txs.is_empty() {
            println!("📦 Batch received: {} transactions", txs.len());
            for (i, tx) in txs.iter().enumerate() {
                println!("  {}. Hash: {}", total_fetched + i + 1, tx.hash);

                // Show detection latency
                let latency_us = tx.detection_ns as f64 / 1000.0;
                println!("     Detection latency: {:.2} μs", latency_us);
            }
            total_fetched += txs.len();
        } else {
            // No transactions, wait a bit
            tokio::time::sleep(Duration::from_millis(100)).await;
            attempts += 1;
            if attempts % 10 == 0 {
                println!(
                    "⏳ Waiting for transactions... (attempt {}/{})",
                    attempts, max_attempts
                );
            }
        }
    }

    // 4. Summary
    println!("\n📊 Test Summary:");
    println!("================");
    if total_fetched > 0 {
        println!("✅ SUCCESS: Fetched {} transactions", total_fetched);
        println!("✅ Mempool fetcher is working correctly!");
    } else {
        println!("⚠️  WARNING: No transactions received");
        println!("    This could mean:");
        println!("    - Mempool is empty (unlikely on mainnet)");
        println!("    - IPC connection issue");
        println!("    - Subscription not working");
    }

    Ok(())
}
