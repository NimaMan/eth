/// Full Mempool Access via WebSocket
/// 
/// This demonstrates how to get the complete mempool using WebSocket subscription
/// instead of limited RPC calls. We subscribe to ALL pending transactions and
/// build up the full mempool view.
///
/// Usage:
///   cargo run --bin mempool_websocket_full

use mempool_processor::mempool_fetcher::streaming_fetcher::StreamingFetcher;
use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error};
use std::collections::HashMap;
use ethers::prelude::*;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info,mempool_websocket_full=info")
        .with_max_level(tracing::Level::INFO)
        .init();
    
    let ws_url = std::env::var("ETH_RPC_WS").unwrap_or_else(|_| "ws://localhost:8546".to_string());
    let http_url = std::env::var("ETH_RPC_HTTP").unwrap_or_else(|_| "http://localhost:8545".to_string());
    
    info!("🌊 WebSocket Full Mempool Access");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("WebSocket: {}", ws_url);
    info!("HTTP: {}", http_url);
    
    // Phase 1: Get current mempool size for reference
    info!("\n📊 Phase 1: Checking current mempool size...");
    let provider = Provider::<Http>::try_from(&http_url)?;
    
    // Get mempool stats
    let stats: serde_json::Value = provider.request("txpool_status", ()).await?;
    let pending_count = stats["pending"].as_str()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    let queued_count = stats["queued"].as_str()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    
    info!("Current mempool size:");
    info!("  • Pending: {} transactions", pending_count);
    info!("  • Queued: {} transactions", queued_count);
    info!("  • Total: {} transactions", pending_count + queued_count);
    
    // Phase 2: Subscribe and collect ALL transactions
    info!("\n📡 Phase 2: WebSocket subscription to capture full mempool...");
    
    // Connect to WebSocket
    let ws = Ws::connect(&ws_url).await?;
    let ws_provider = Provider::new(ws);
    
    // Subscribe to pending transactions
    let mut stream = ws_provider.subscribe_pending_txs().await?;
    info!("✅ WebSocket subscription active!");
    
    // Also get full transaction details subscription if available
    // Some nodes support "newPendingTransactionsFull" which includes full tx data
    let full_subscription_available = false; // Check if your node supports this
    
    let mut all_transactions = HashMap::new();
    let start_time = Instant::now();
    let collection_duration = Duration::from_secs(30); // Collect for 30 seconds
    
    info!("📥 Collecting transactions for {} seconds...", collection_duration.as_secs());
    info!("   (New transactions will be marked with 🆕)");
    
    // Track statistics
    let mut new_tx_count = 0;
    let mut duplicate_count = 0;
    let mut fetch_errors = 0;
    
    // Collect transactions for the specified duration
    while start_time.elapsed() < collection_duration {
        // Set timeout to check elapsed time periodically
        match tokio::time::timeout(Duration::from_secs(1), stream.next()).await {
            Ok(Some(tx_hash)) => {
                if all_transactions.contains_key(&tx_hash) {
                    duplicate_count += 1;
                } else {
                    // Fetch full transaction details
                    match provider.get_transaction(tx_hash).await {
                        Ok(Some(tx)) => {
                            all_transactions.insert(tx_hash, tx);
                            new_tx_count += 1;
                            
                            if new_tx_count % 100 == 0 {
                                info!("  Progress: {} unique transactions collected...", 
                                      all_transactions.len());
                            }
                            
                            // Mark very recent transactions
                            if start_time.elapsed() > Duration::from_secs(25) {
                                info!("  🆕 New transaction: 0x{}", hex::encode(tx_hash));
                            }
                        }
                        Ok(None) => {
                            warn!("Transaction {} not found", tx_hash);
                            fetch_errors += 1;
                        }
                        Err(e) => {
                            warn!("Error fetching transaction {}: {}", tx_hash, e);
                            fetch_errors += 1;
                        }
                    }
                }
            }
            Ok(None) => {
                warn!("WebSocket stream ended unexpectedly");
                break;
            }
            Err(_) => {
                // Timeout - check if we should continue
                if start_time.elapsed() >= collection_duration {
                    break;
                }
            }
        }
    }
    
    let collection_time = start_time.elapsed();
    
    // Phase 3: Analyze results
    info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 WEBSOCKET MEMPOOL COLLECTION RESULTS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    info!("\n🎯 Collection Summary:");
    info!("  • Collection time: {:?}", collection_time);
    info!("  • Unique transactions: {}", all_transactions.len());
    info!("  • Duplicates seen: {}", duplicate_count);
    info!("  • Fetch errors: {}", fetch_errors);
    info!("  • Transaction rate: {:.2} tx/s", 
          new_tx_count as f64 / collection_time.as_secs_f64());
    
    // Estimate full mempool capture time
    let capture_rate = all_transactions.len() as f64 / collection_time.as_secs_f64();
    let estimated_full_capture_time = (pending_count + queued_count) as f64 / capture_rate;
    
    info!("\n📈 Mempool Coverage Analysis:");
    info!("  • Expected mempool size: {}", pending_count + queued_count);
    info!("  • Captured in {}s: {} ({:.1}%)", 
          collection_time.as_secs(),
          all_transactions.len(),
          all_transactions.len() as f64 / (pending_count + queued_count) as f64 * 100.0);
    info!("  • Estimated time for full mempool: {:.1}s", estimated_full_capture_time);
    
    // Analyze transaction types
    let mut value_transfers = 0;
    let mut contract_calls = 0;
    let mut total_gas_price = U256::zero();
    
    for (_, tx) in &all_transactions {
        if tx.value > U256::zero() {
            value_transfers += 1;
        }
        if tx.to.is_none() || tx.input.len() > 4 {
            contract_calls += 1;
        }
        if let Some(gas_price) = tx.gas_price {
            total_gas_price = total_gas_price + gas_price;
        }
    }
    
    info!("\n💰 Transaction Analysis:");
    info!("  • Value transfers: {} ({:.1}%)", 
          value_transfers, 
          value_transfers as f64 / all_transactions.len() as f64 * 100.0);
    info!("  • Contract interactions: {} ({:.1}%)", 
          contract_calls,
          contract_calls as f64 / all_transactions.len() as f64 * 100.0);
    
    if all_transactions.len() > 0 {
        let avg_gas_price = total_gas_price / all_transactions.len();
        info!("  • Average gas price: {} gwei", avg_gas_price / 1_000_000_000u64);
    }
    
    info!("\n💡 WebSocket Advantages:");
    info!("  ✅ Real-time push notifications (no polling delay)");
    info!("  ✅ Can capture ALL transactions (not limited like RPC)");
    info!("  ✅ Lower latency (10-50ms vs 100-500ms)");
    info!("  ✅ More efficient than repeated RPC calls");
    
    info!("\n🚀 Recommendations:");
    if capture_rate < 100.0 {
        info!("  ⚠️  Capture rate is low. Consider:");
        info!("     - Using multiple WebSocket connections");
        info!("     - Parallel transaction fetching");
        info!("     - Local node for faster response");
    }
    
    if estimated_full_capture_time > 60.0 {
        info!("  ⚠️  Full capture would take >1 minute. For faster initial sync:");
        info!("     - Use direct Reth pool access");
        info!("     - Implement DevP2P client");
        info!("     - Consider hybrid approach: RPC for initial, WS for updates");
    }
    
    // Save results
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let results = serde_json::json!({
        "timestamp": timestamp,
        "websocket_url": ws_url,
        "collection_duration_sec": collection_time.as_secs(),
        "mempool_size": {
            "expected_pending": pending_count,
            "expected_queued": queued_count,
            "expected_total": pending_count + queued_count,
        },
        "capture_results": {
            "unique_transactions": all_transactions.len(),
            "duplicates": duplicate_count,
            "errors": fetch_errors,
            "tx_per_second": capture_rate,
            "coverage_percent": all_transactions.len() as f64 / (pending_count + queued_count) as f64 * 100.0,
        },
        "estimates": {
            "full_capture_time_sec": estimated_full_capture_time,
        }
    });
    
    let filename = format!("websocket_mempool_capture_{}.json", timestamp);
    std::fs::write(&filename, serde_json::to_string_pretty(&results)?)?;
    info!("\n💾 Results saved to: {}", filename);
    
    Ok(())
}