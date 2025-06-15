/// Measure Mempool Transaction Fetch Timing
/// 
/// This binary measures:
/// 1. Initial mempool fetch timing (thousands of existing transactions)
/// 2. Real-time monitoring latency
/// 3. Transition between phases
///
/// Usage:
///   cargo run --bin measure_mempool_timing

use mempool_processor::mempool_fetcher::{MempoolFetcher, TransactionSource};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info,measure_mempool_timing=info")
        .with_max_level(tracing::Level::INFO)
        .init();
    
    let http_url = std::env::var("ETH_RPC_HTTP").unwrap_or_else(|_| "http://localhost:8545".to_string());
    
    info!("🔍 Mempool Transaction Fetch Timing Measurement");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("RPC Endpoint: {}", http_url);
    
    // Create a simple fetcher - using the actual constructor
    let fetcher = MempoolFetcher::new(&http_url)?;
    
    let overall_start = Instant::now();
    let mut total_txs = 0;
    let mut batch_count = 0;
    let mut consecutive_empty = 0;
    let mut tx_cache = HashMap::new(); // Track processed transactions
    
    info!("\n📥 PHASE 1: Fetching Initial Mempool State");
    info!("This represents the thousands of transactions already in mempool...\n");
    
    let phase1_start = Instant::now();
    
    // Fetch until we get consecutive empty results
    loop {
        let batch_start = Instant::now();
        
        match fetcher.get_transactions().await {
            Ok(transactions) => {
                let batch_time = batch_start.elapsed();
                
                if transactions.is_empty() {
                    consecutive_empty += 1;
                    if consecutive_empty >= 3 {
                        info!("\n✅ Initial fetch complete (3 consecutive empty results)");
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    continue;
                }
                
                // Reset counter
                consecutive_empty = 0;
                batch_count += 1;
                
                // Filter out duplicates
                let new_txs: Vec<_> = transactions.into_iter()
                    .filter(|tx| {
                        let hash = hex::encode(&tx.hash);
                        if tx_cache.contains_key(&hash) {
                            false
                        } else {
                            tx_cache.insert(hash, Instant::now());
                            true
                        }
                    })
                    .collect();
                
                let new_count = new_txs.len();
                total_txs += new_count;
                
                if new_count > 0 {
                    let ms_per_tx = batch_time.as_millis() as f64 / new_count as f64;
                    
                    info!(
                        "Batch {:02}: {} new txs in {:?} ({:.2} ms/tx)",
                        batch_count, new_count, batch_time, ms_per_tx
                    );
                    
                    // Show sample transaction
                    if let Some(tx) = new_txs.first() {
                        info!(
                            "  └─ Sample: {} value: {} wei",
                            &hex::encode(&tx.hash)[..16],
                            tx.value
                        );
                    }
                }
                
                // Stop after reasonable amount for demo
                if total_txs >= 500 {
                    info!("\n✅ Reached 500 transaction limit for demo");
                    break;
                }
            }
            Err(e) => {
                warn!("Error fetching batch: {}", e);
                break;
            }
        }
    }
    
    let phase1_time = phase1_start.elapsed();
    
    info!("\n📊 Phase 1 Summary:");
    info!("  • Total unique transactions: {}", total_txs);
    info!("  • Total batches: {}", batch_count);
    info!("  • Total time: {:?}", phase1_time);
    info!("  • Average rate: {:.2} tx/s", 
        total_txs as f64 / phase1_time.as_secs_f64()
    );
    
    // Phase 2: Monitor for new transactions
    info!("\n🚀 PHASE 2: Real-time Monitoring");
    info!("Simulating continuous monitoring for new transactions...\n");
    
    let phase2_start = Instant::now();
    let mut new_tx_count = 0;
    let mut poll_count = 0;
    let mut polls_with_data = 0;
    
    // Monitor for 10 seconds
    while phase2_start.elapsed().as_secs() < 10 {
        poll_count += 1;
        let poll_start = Instant::now();
        
        match fetcher.get_transactions().await {
            Ok(transactions) => {
                // Filter duplicates
                let new_txs: Vec<_> = transactions.into_iter()
                    .filter(|tx| {
                        let hash = hex::encode(&tx.hash);
                        !tx_cache.contains_key(&hash)
                    })
                    .collect();
                
                if !new_txs.is_empty() {
                    polls_with_data += 1;
                    new_tx_count += new_txs.len();
                    let poll_time = poll_start.elapsed();
                    
                    info!(
                        "Poll {:02}: {} new txs detected in {:?} (latency: {:?}/tx)",
                        poll_count,
                        new_txs.len(),
                        poll_time,
                        poll_time / new_txs.len() as u32
                    );
                    
                    // Add to cache
                    for tx in new_txs {
                        tx_cache.insert(hex::encode(&tx.hash), Instant::now());
                    }
                }
            }
            Err(e) => {
                warn!("Poll error: {}", e);
            }
        }
        
        // Wait before next poll
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    let total_time = overall_start.elapsed();
    
    // Final summary
    info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 FINAL TIMING SUMMARY");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    info!("\n🎯 Initial Mempool Fetch (Phase 1):");
    info!("  • Transactions: {} unique", total_txs);
    info!("  • Duration: {:?}", phase1_time);
    info!("  • Rate: {:.2} tx/s", total_txs as f64 / phase1_time.as_secs_f64());
    
    info!("\n🚀 Real-time Monitoring (Phase 2):");
    info!("  • New transactions: {}", new_tx_count);
    info!("  • Total polls: {}", poll_count);
    info!("  • Polls with data: {} ({:.1}%)", 
        polls_with_data,
        polls_with_data as f64 / poll_count as f64 * 100.0
    );
    info!("  • Avg new tx/poll: {:.2}", new_tx_count as f64 / poll_count as f64);
    
    info!("\n⏱️  Total runtime: {:?}", total_time);
    info!("💾 Cache size: {} unique transactions", tx_cache.len());
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Save results
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let results = serde_json::json!({
        "timestamp": timestamp,
        "endpoint": http_url,
        "phase1_initial": {
            "unique_transactions": total_txs,
            "duration_ms": phase1_time.as_millis(),
            "rate_tx_per_sec": total_txs as f64 / phase1_time.as_secs_f64(),
        },
        "phase2_monitoring": {
            "duration_s": 10,
            "new_transactions": new_tx_count,
            "polls": poll_count,
            "polls_with_data": polls_with_data,
        },
        "total": {
            "runtime_ms": total_time.as_millis(),
            "unique_transactions": tx_cache.len(),
        }
    });
    
    let filename = format!("mempool_timing_{}.json", timestamp);
    std::fs::write(&filename, serde_json::to_string_pretty(&results)?)?;
    info!("\n💾 Detailed results saved to: {}", filename);
    
    Ok(())
}