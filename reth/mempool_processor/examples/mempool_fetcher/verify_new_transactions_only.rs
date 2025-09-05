/// Verify that we only receive NEW transactions entering the mempool
/// 
/// This test will:
/// 1. Get current mempool size
/// 2. Start subscription
/// 3. Track first 100 transactions
/// 4. Check if we see any existing transactions or only new ones

use mempool_processor::mempool_fetcher::MempoolFetcherIPCClient;
use ethers::providers::{Provider, Http, Middleware};
use std::collections::HashSet;
use std::time::{Instant, Duration};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();
    
    info!("Verifying that we only receive NEW transactions...");
    
    // First, get some existing transactions from mempool
    let provider = Provider::<Http>::try_from("http://localhost:8545")?;
    
    // Get current mempool content
    info!("Getting current mempool transactions...");
    let mempool_content: serde_json::Value = provider
        .request("txpool_content", ())
        .await?;
    
    // Extract some transaction hashes from existing mempool
    let mut existing_hashes = HashSet::new();
    if let Some(pending) = mempool_content.get("pending") {
        for (_, txs_by_nonce) in pending.as_object().unwrap() {
            for (_, tx) in txs_by_nonce.as_object().unwrap() {
                if let Some(hash) = tx.get("hash").and_then(|h| h.as_str()) {
                    existing_hashes.insert(hash.to_string());
                }
            }
        }
    }
    
    info!("Found {} existing transactions in mempool", existing_hashes.len());
    
    // Now start our subscription
    info!("Starting subscription to newPendingTransactions...");
    let ipc_client = MempoolFetcherIPCClient::new(Some("/tmp/reth.ipc"))?;
    ipc_client.start().await?;
    
    // Track what we receive
    let mut received_count = 0;
    let mut new_count = 0;
    let mut existing_count = 0;
    let start = Instant::now();
    
    info!("Monitoring for 30 seconds...");
    
    while start.elapsed() < Duration::from_secs(30) {
        let transactions = ipc_client.get_transactions_instant(100).await;
        if !transactions.is_empty() {
            for tx in transactions {
                received_count += 1;
                
                if existing_hashes.contains(&tx.hash) {
                    existing_count += 1;
                    info!("❌ EXISTING transaction detected: {}", &tx.hash[..10]);
                } else {
                    new_count += 1;
                    if new_count <= 5 {
                        info!("✅ NEW transaction: {} ({}μs)", 
                              &tx.hash[..10], 
                              tx.detection_ns / 1000);
                    }
                }
                
                if received_count % 50 == 0 {
                    info!("Progress: {} total, {} new, {} existing", 
                          received_count, new_count, existing_count);
                }
            }
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    info!("\n=== VERIFICATION RESULTS ===");
    info!("Total transactions received: {}", received_count);
    info!("NEW transactions: {} ({:.1}%)", 
          new_count, 
          (new_count as f64 / received_count as f64) * 100.0);
    info!("EXISTING transactions: {} ({:.1}%)", 
          existing_count,
          (existing_count as f64 / received_count as f64) * 100.0);
    
    if existing_count == 0 {
        info!("\n✅ SUCCESS: We only receive NEW transactions!");
        info!("The 'newPendingTransactions' subscription works as expected.");
    } else {
        info!("\n❌ UNEXPECTED: We received some existing transactions!");
        info!("This would indicate a problem with our understanding.");
    }
    
    Ok(())
}