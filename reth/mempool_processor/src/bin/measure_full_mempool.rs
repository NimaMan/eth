/// Measure Full Mempool Fetch
/// 
/// This binary measures the complete mempool fetch including both pending and queued transactions.
/// It shows the true size of the mempool and measures how long it takes to fetch all transactions.
///
/// Usage:
///   cargo run --bin measure_full_mempool

use mempool_processor::mempool_fetcher::{MempoolFetcher, TransactionSource};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info,measure_full_mempool=info")
        .with_max_level(tracing::Level::INFO)
        .init();
    
    let http_url = std::env::var("ETH_RPC_HTTP").unwrap_or_else(|_| "http://localhost:8545".to_string());
    
    info!("🔍 Full Mempool Measurement");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("RPC Endpoint: {}", http_url);
    
    // Create fetcher with large cache to handle full mempool
    let fetcher = MempoolFetcher::new(&http_url)?;
    
    // First, get the actual mempool size
    info!("\n📊 Checking mempool status...");
    let stats_start = Instant::now();
    let (pending_count, queued_count) = fetcher.get_stats().await?;
    let stats_time = stats_start.elapsed();
    
    info!("Mempool Status (fetched in {:?}):", stats_time);
    info!("  • Pending transactions: {}", pending_count);
    info!("  • Queued transactions: {}", queued_count);
    info!("  • Total: {}", pending_count + queued_count);
    
    // Now fetch all transactions
    info!("\n📥 Fetching complete mempool...");
    info!("Note: Default batch size is 100, so we'll need {} calls minimum", 
        (pending_count + queued_count + 99) / 100);
    
    let fetch_start = Instant::now();
    let mut all_transactions = HashMap::new();
    let mut batch_count = 0;
    let mut total_fetch_time = std::time::Duration::from_secs(0);
    
    loop {
        batch_count += 1;
        let batch_start = Instant::now();
        
        match fetcher.get_transactions().await {
            Ok(transactions) => {
                let batch_time = batch_start.elapsed();
                total_fetch_time += batch_time;
                
                if transactions.is_empty() {
                    info!("✅ Mempool exhausted after {} batches", batch_count);
                    break;
                }
                
                // Add to our collection
                let mut new_count = 0;
                for tx in transactions {
                    let hash = hex::encode(&tx.hash);
                    if !all_transactions.contains_key(&hash) {
                        all_transactions.insert(hash, tx);
                        new_count += 1;
                    }
                }
                
                info!(
                    "Batch {:03}: {} transactions ({} new) in {:?}",
                    batch_count, 
                    new_count + (all_transactions.len() - new_count),
                    new_count,
                    batch_time
                );
                
                // Show progress
                if batch_count % 10 == 0 {
                    info!("  Progress: {} unique transactions fetched so far...", all_transactions.len());
                }
                
                // Safety limit
                if batch_count > 1000 {
                    warn!("Safety limit reached (1000 batches)");
                    break;
                }
            }
            Err(e) => {
                warn!("Error fetching batch {}: {}", batch_count, e);
                break;
            }
        }
    }
    
    let total_time = fetch_start.elapsed();
    
    // Analyze results
    info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 FULL MEMPOOL FETCH RESULTS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    info!("\n🎯 Mempool Size:");
    info!("  • Expected: {} (pending: {}, queued: {})", 
        pending_count + queued_count, pending_count, queued_count);
    info!("  • Actually fetched: {} unique transactions", all_transactions.len());
    info!("  • Difference: {}", 
        (pending_count + queued_count) as i64 - all_transactions.len() as i64);
    
    info!("\n⏱️  Timing:");
    info!("  • Total batches: {}", batch_count);
    info!("  • Total time: {:?}", total_time);
    info!("  • Avg per batch: {:?}", total_time / batch_count as u32);
    info!("  • Avg per transaction: {:?}", 
        if all_transactions.len() > 0 { total_time / all_transactions.len() as u32 } else { total_time });
    info!("  • Transaction rate: {:.2} tx/s", 
        all_transactions.len() as f64 / total_time.as_secs_f64());
    
    // Analyze transaction types
    let mut value_txs = 0;
    let mut contract_txs = 0;
    let mut total_value = ethers::types::U256::zero();
    
    for (_, tx) in &all_transactions {
        if tx.value > ethers::types::U256::zero() {
            value_txs += 1;
            total_value = total_value + tx.value;
        }
        if tx.to.is_none() || (tx.input_data.is_some() && tx.input_data.as_ref().unwrap().len() > 4) {
            contract_txs += 1;
        }
    }
    
    info!("\n📈 Transaction Analysis:");
    info!("  • Value transfers: {} ({:.1}%)", 
        value_txs, value_txs as f64 / all_transactions.len() as f64 * 100.0);
    info!("  • Contract interactions: {} ({:.1}%)", 
        contract_txs, contract_txs as f64 / all_transactions.len() as f64 * 100.0);
    info!("  • Total ETH value: {} wei", total_value);
    
    info!("\n💡 Recommendations:");
    if batch_count > 50 {
        info!("  ⚠️  Too many batches needed! Consider:");
        info!("     - Increasing batch size from 100 to 1000");
        info!("     - Using WebSocket subscription for efficiency");
        info!("     - Implementing parallel batch fetching");
    }
    
    if all_transactions.len() < (pending_count + queued_count) {
        info!("  ⚠️  Didn't fetch all transactions! Possible causes:");
        info!("     - Transactions being mined during fetch");
        info!("     - Cache limiting repeated fetches");
        info!("     - Batch size too small");
    }
    
    // Save detailed results
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let results = serde_json::json!({
        "timestamp": timestamp,
        "endpoint": http_url,
        "mempool_status": {
            "pending": pending_count,
            "queued": queued_count,
            "total_expected": pending_count + queued_count,
        },
        "fetch_results": {
            "transactions_fetched": all_transactions.len(),
            "batches": batch_count,
            "total_time_ms": total_time.as_millis(),
            "avg_per_tx_us": if all_transactions.len() > 0 { 
                total_time.as_micros() / all_transactions.len() as u128 
            } else { 0 },
            "rate_tx_per_sec": all_transactions.len() as f64 / total_time.as_secs_f64(),
        },
        "analysis": {
            "value_transfers": value_txs,
            "contract_interactions": contract_txs,
            "total_eth_value": total_value.to_string(),
        }
    });
    
    let filename = format!("full_mempool_measurement_{}.json", timestamp);
    std::fs::write(&filename, serde_json::to_string_pretty(&results)?)?;
    info!("\n💾 Detailed results saved to: {}", filename);
    
    Ok(())
}