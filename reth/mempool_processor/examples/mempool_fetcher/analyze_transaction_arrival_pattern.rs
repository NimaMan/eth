/// Analyze Transaction Arrival Pattern
/// 
/// Monitors how transactions actually arrive from the mempool
/// to understand burst patterns and timing
///
/// Run with: cargo run --example analyze_transaction_arrival_pattern --release

use std::time::{Duration, Instant};
use std::collections::VecDeque;
use mempool_processor::mempool_fetcher::NonBlockingIpcClient;
use tracing::info;
use tracing_subscriber;

#[derive(Debug, Clone)]
struct ArrivalBurst {
    timestamp: Instant,
    count: usize,
    detection_ns_avg: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    info!("🔍 Analyzing Transaction Arrival Pattern");
    
    // Initialize client
    let client = NonBlockingIpcClient::new(Some("/tmp/reth.ipc"))?;
    client.start().await?;
    
    info!("Client started, monitoring for 60 seconds...");
    
    let start_time = Instant::now();
    let test_duration = Duration::from_secs(60);
    
    // Track arrival bursts
    let mut bursts: Vec<ArrivalBurst> = Vec::new();
    let mut current_burst_start: Option<Instant> = None;
    let mut current_burst_txs = 0;
    let mut current_burst_detection_sum = 0u64;
    
    // Track inter-arrival times
    let mut last_tx_time: Option<Instant> = None;
    let mut inter_arrival_times: VecDeque<Duration> = VecDeque::new();
    
    // Continuous monitoring stats
    let mut total_txs = 0u64;
    let mut empty_fetch_streak = 0u64;
    let mut max_empty_streak = 0u64;
    let mut fetch_count = 0u64;
    
    // Use very fast polling to catch all bursts
    let mut consecutive_empties = 0;
    
    while start_time.elapsed() < test_duration {
        fetch_count += 1;
        
        // Get transactions with minimal delay
        let txs = client.get_transactions_instant(1000).await; // Large buffer to catch bursts
        
        if txs.is_empty() {
            empty_fetch_streak += 1;
            consecutive_empties += 1;
            
            // End burst if we've been empty for >10ms
            if consecutive_empties > 100 && current_burst_start.is_some() {
                if current_burst_txs > 0 {
                    bursts.push(ArrivalBurst {
                        timestamp: current_burst_start.unwrap(),
                        count: current_burst_txs,
                        detection_ns_avg: current_burst_detection_sum / current_burst_txs as u64,
                    });
                }
                current_burst_start = None;
                current_burst_txs = 0;
                current_burst_detection_sum = 0;
            }
            
            // Very short sleep
            tokio::time::sleep(Duration::from_micros(100)).await;
        } else {
            // Got transactions!
            let now = Instant::now();
            consecutive_empties = 0;
            
            // Track empty streak
            if empty_fetch_streak > max_empty_streak {
                max_empty_streak = empty_fetch_streak;
            }
            empty_fetch_streak = 0;
            
            // Start new burst if needed
            if current_burst_start.is_none() {
                current_burst_start = Some(now);
            }
            
            // Add to current burst
            current_burst_txs += txs.len();
            let detection_sum: u64 = txs.iter().map(|tx| tx.detection_ns).sum();
            current_burst_detection_sum += detection_sum;
            
            // Track inter-arrival time
            if let Some(last_time) = last_tx_time {
                let inter_arrival = now.duration_since(last_time);
                inter_arrival_times.push_back(inter_arrival);
                if inter_arrival_times.len() > 1000 {
                    inter_arrival_times.pop_front();
                }
            }
            last_tx_time = Some(now);
            
            total_txs += txs.len() as u64;
            
            // Log large bursts immediately
            if txs.len() >= 10 {
                info!("🌊 BURST: {} transactions arrived together!", txs.len());
            }
        }
    }
    
    // Close final burst
    if current_burst_start.is_some() && current_burst_txs > 0 {
        bursts.push(ArrivalBurst {
            timestamp: current_burst_start.unwrap(),
            count: current_burst_txs,
            detection_ns_avg: current_burst_detection_sum / current_burst_txs as u64,
        });
    }
    
    // Analyze results
    let elapsed = start_time.elapsed();
    let avg_rate = total_txs as f64 / elapsed.as_secs_f64();
    
    info!("\n📊 ARRIVAL PATTERN ANALYSIS (60 seconds)");
    info!("Total transactions: {}", total_txs);
    info!("Average rate: {:.1} tx/sec", avg_rate);
    info!("Total fetches: {}", fetch_count);
    info!("Max empty streak: {} fetches ({:.1}ms)", 
          max_empty_streak, 
          max_empty_streak as f64 * 0.1);
    
    // Burst analysis
    if !bursts.is_empty() {
        let total_burst_txs: usize = bursts.iter().map(|b| b.count).sum();
        let avg_burst_size = total_burst_txs as f64 / bursts.len() as f64;
        let max_burst = bursts.iter().map(|b| b.count).max().unwrap_or(0);
        
        info!("\n🌊 BURST PATTERNS:");
        info!("Total bursts: {}", bursts.len());
        info!("Average burst size: {:.1} transactions", avg_burst_size);
        info!("Maximum burst size: {} transactions", max_burst);
        info!("Transactions in bursts: {} ({:.1}%)", 
              total_burst_txs,
              total_burst_txs as f64 / total_txs as f64 * 100.0);
        
        // Show largest bursts
        let mut sorted_bursts = bursts.clone();
        sorted_bursts.sort_by_key(|b| std::cmp::Reverse(b.count));
        info!("\nTop 5 largest bursts:");
        for (i, burst) in sorted_bursts.iter().take(5).enumerate() {
            info!("  {}. {} transactions (avg detection: {}μs)", 
                  i + 1, 
                  burst.count,
                  burst.detection_ns_avg / 1000);
        }
    }
    
    // Inter-arrival analysis
    if !inter_arrival_times.is_empty() {
        let mut sorted_inter: Vec<_> = inter_arrival_times.iter()
            .map(|d| d.as_micros() as u64)
            .collect();
        sorted_inter.sort();
        
        let p50 = sorted_inter[sorted_inter.len() / 2];
        let p95 = sorted_inter[sorted_inter.len() * 95 / 100];
        let p99 = sorted_inter[sorted_inter.len() * 99 / 100];
        
        info!("\n⏱️  INTER-ARRIVAL TIMES:");
        info!("P50: {}ms", p50 / 1000);
        info!("P95: {}ms", p95 / 1000);
        info!("P99: {}ms", p99 / 1000);
    }
    
    // Recommendations
    info!("\n💡 RECOMMENDATIONS:");
    if !bursts.is_empty() {
        let total_burst_txs: usize = bursts.iter().map(|b| b.count).sum();
        let avg_burst_size = total_burst_txs as f64 / bursts.len() as f64;
        
        if avg_burst_size > 10.0 {
            info!("- Transactions arrive in bursts of ~{:.0} - increase batch size", avg_burst_size);
        }
        if total_burst_txs as f64 / total_txs as f64 > 0.8 {
            info!("- {:.0}% of transactions arrive in bursts - optimize for burst handling", 
                  total_burst_txs as f64 / total_txs as f64 * 100.0);
        }
    }
    if max_empty_streak > 1000 {
        info!("- Long empty periods ({:.0}ms) - reduce polling frequency when idle", 
              max_empty_streak as f64 * 0.1);
    }
    
    Ok(())
}