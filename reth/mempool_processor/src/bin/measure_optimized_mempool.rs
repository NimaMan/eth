/// Optimized Mempool Fetch with Larger Batch Size
/// 
/// This binary demonstrates how to efficiently fetch the full mempool by using
/// a larger batch size (1000 instead of default 100).
///
/// Usage:
///   cargo run --bin measure_optimized_mempool

use mempool_processor::mempool_fetcher::{MempoolStreamer, StreamerConfig, StreamMode};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info,measure_optimized_mempool=info")
        .with_max_level(tracing::Level::INFO)
        .init();
    
    let http_url = std::env::var("ETH_RPC_HTTP").unwrap_or_else(|_| "http://localhost:8545".to_string());
    
    info!("🚀 Optimized Mempool Fetch Measurement");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("RPC Endpoint: {}", http_url);
    
    // Create optimized streaming configuration
    let config = StreamerConfig {
        ws_url: Some(http_url.replace("http://", "ws://").replace(":8545", ":8546")),
        http_url: http_url.clone(),
        preferred_mode: StreamMode::WebSocket,
        track_latency: true,
        initial_capture_duration: std::time::Duration::from_secs(60),
        ..Default::default()
    };
    
    let streamer = MempoolStreamer::new(config).await?;
    
    // First, get the actual mempool size
    info!("\n📊 Checking mempool status...");
    let stats_start = Instant::now();
    let (pending_count, queued_count) = fetcher.get_stats().await?;
    let stats_time = stats_start.elapsed();
    
    info!("Mempool Status (fetched in {:?}):", stats_time);
    info!("  • Pending transactions: {}", pending_count);
    info!("  • Queued transactions: {}", queued_count);
    info!("  • Total: {}", pending_count + queued_count);
    
    // Now fetch all transactions with optimized settings
    info!("\n📥 Fetching complete mempool with optimized batch size...");
    info!("Expected batches needed: {} (with 1000 batch size)", 
        (pending_count + queued_count + 999) / 1000);
    
    let fetch_start = Instant::now();
    let mut all_transactions = HashMap::new();
    let mut batch_count = 0;
    let mut total_fetch_time = std::time::Duration::from_secs(0);
    
    // Comparison metrics
    let mut batch_sizes = Vec::new();
    let mut batch_times = Vec::new();
    
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
                
                let batch_size = transactions.len();
                batch_sizes.push(batch_size);
                batch_times.push(batch_time.as_millis() as u64);
                
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
                    "Batch {:03}: {} transactions ({} new) in {:?} = {:.2} ms/tx",
                    batch_count, 
                    batch_size,
                    new_count,
                    batch_time,
                    batch_time.as_millis() as f64 / batch_size as f64
                );
                
                // Show progress
                if batch_count % 5 == 0 {
                    info!("  Progress: {} unique transactions fetched so far...", all_transactions.len());
                }
                
                // Safety limit
                if batch_count > 100 {
                    warn!("Safety limit reached (100 batches)");
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
    
    // Calculate statistics
    let avg_batch_size = batch_sizes.iter().sum::<usize>() as f64 / batch_sizes.len() as f64;
    let max_batch_size = batch_sizes.iter().max().unwrap_or(&0);
    let avg_batch_time = batch_times.iter().sum::<u64>() as f64 / batch_times.len() as f64;
    
    // Analyze results
    info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 OPTIMIZED MEMPOOL FETCH RESULTS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    info!("\n🎯 Mempool Coverage:");
    info!("  • Expected: {} transactions", pending_count + queued_count);
    info!("  • Fetched: {} unique transactions", all_transactions.len());
    info!("  • Coverage: {:.1}%", 
        all_transactions.len() as f64 / (pending_count + queued_count) as f64 * 100.0);
    
    info!("\n⏱️  Timing Performance:");
    info!("  • Total batches: {}", batch_count);
    info!("  • Total time: {:?}", total_time);
    info!("  • Avg batch size: {:.1} transactions", avg_batch_size);
    info!("  • Max batch size: {} transactions", max_batch_size);
    info!("  • Avg batch time: {:.1} ms", avg_batch_time);
    info!("  • Transaction rate: {:.2} tx/s", 
        all_transactions.len() as f64 / total_time.as_secs_f64());
    
    info!("\n🚀 Optimization Impact:");
    
    // Compare with default settings (100 batch size)
    let default_batches = (all_transactions.len() + 99) / 100;
    let default_time_estimate = default_batches as f64 * avg_batch_time / 1000.0;
    
    info!("  • Default (100 batch size):");
    info!("    - Would need {} batches", default_batches);
    info!("    - Estimated time: {:.1} seconds", default_time_estimate);
    
    info!("  • Optimized (1000 batch size):");
    info!("    - Used {} batches", batch_count);
    info!("    - Actual time: {:.1} seconds", total_time.as_secs_f64());
    
    let speedup = default_time_estimate / total_time.as_secs_f64();
    info!("  • Speedup: {:.1}x faster", speedup);
    
    info!("\n💾 Efficiency Metrics:");
    let efficiency = all_transactions.len() as f64 / (batch_count as f64 * 1000.0) * 100.0;
    info!("  • Batch efficiency: {:.1}% (avg {} of 1000 capacity used)", 
        efficiency, avg_batch_size as i32);
    
    // Recommendations
    info!("\n💡 Recommendations:");
    if avg_batch_size < 800.0 {
        info!("  ⚠️  Batch size not fully utilized. Consider:");
        info!("     - Current mempool might be smaller than batch size");
        info!("     - Could use dynamic batch sizing");
    }
    
    if total_time.as_secs() > 10 {
        info!("  ⚠️  Still taking >10s for initial fetch. Consider:");
        info!("     - WebSocket subscription for real-time updates");
        info!("     - Parallel batch fetching");
        info!("     - DevP2P for direct mempool access");
    }
    
    // Save comparison results
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let results = serde_json::json!({
        "timestamp": timestamp,
        "endpoint": http_url,
        "optimization": {
            "batch_size": 1000,
            "cache_size": 2_000_000,
        },
        "results": {
            "transactions_fetched": all_transactions.len(),
            "batches": batch_count,
            "total_time_ms": total_time.as_millis(),
            "avg_batch_size": avg_batch_size,
            "max_batch_size": max_batch_size,
            "tx_per_second": all_transactions.len() as f64 / total_time.as_secs_f64(),
        },
        "comparison": {
            "default_batch_size": 100,
            "default_estimated_batches": default_batches,
            "default_estimated_time_sec": default_time_estimate,
            "speedup_factor": speedup,
        }
    });
    
    let filename = format!("optimized_mempool_measurement_{}.json", timestamp);
    std::fs::write(&filename, serde_json::to_string_pretty(&results)?)?;
    info!("\n💾 Results saved to: {}", filename);
    
    Ok(())
}