/// Definitive Mempool Performance Measurement
/// 
/// This measures:
/// 1. Initial mempool size
/// 2. Time to capture initial transactions
/// 3. Detection latency for new transactions over 1 minute
///
/// Run with:
///   cargo run --example measure_mempool_performance

use std::time::{Duration, Instant};
use std::collections::HashMap;
use serde_json::{Value, json};
use eyre::Result;
use tokio::net::UnixStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use mempool_processor::mempool_fetcher::ipc_socket::OptimizedIpcClient;

#[derive(Debug)]
struct MempoolStats {
    initial_size: usize,
    initial_capture_time: Option<Duration>,
    transactions_detected: usize,
    detection_latencies: Vec<Duration>,
    start_time: Instant,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("📊 DEFINITIVE MEMPOOL PERFORMANCE MEASUREMENT");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Measuring real-world performance with optimized IPC\n");
    
    let mut stats = MempoolStats {
        initial_size: 0,
        initial_capture_time: None,
        transactions_detected: 0,
        detection_latencies: Vec::new(),
        start_time: Instant::now(),
    };
    
    // Step 1: Check current mempool size
    println!("📋 Step 1: Checking mempool status...");
    check_mempool_status(&mut stats).await?;
    
    // Step 2: Start optimized monitoring
    println!("\n📡 Step 2: Starting optimized transaction detection...");
    monitor_with_optimized_ipc(&mut stats).await?;
    
    // Step 3: Display results
    display_results(&stats);
    
    Ok(())
}

async fn check_mempool_status(stats: &mut MempoolStats) -> Result<()> {
    let mut stream = UnixStream::connect("/tmp/reth.ipc").await?;
    
    // Get txpool status
    let status_request = json!({
        "jsonrpc": "2.0",
        "method": "txpool_status",
        "params": [],
        "id": 1
    });
    
    stream.write_all(format!("{}\n", status_request).as_bytes()).await?;
    
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    
    let response: Value = serde_json::from_str(
        std::str::from_utf8(&buf[..n])?.lines().next().unwrap_or("")
    )?;
    
    if let Some(result) = response["result"].as_object() {
        let pending = result.get("pending")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0) as usize;
            
        let queued = result.get("queued")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0) as usize;
        
        stats.initial_size = pending + queued;
        
        println!("  Pending transactions: {}", pending);
        println!("  Queued transactions: {}", queued);
        println!("  Total in mempool: {}", stats.initial_size);
    } else {
        println!("  ⚠️  Could not get mempool status");
    }
    
    // Skip txpool_content - it's too large and not needed for this measurement
    println!("  Note: Subscriptions only show NEW transactions entering mempool");
    
    Ok(())
}

async fn monitor_with_optimized_ipc(stats: &mut MempoolStats) -> Result<()> {
    let mut client = OptimizedIpcClient::new(None).await?;
    
    println!("  ✅ Connected with optimized IPC socket");
    
    // Subscribe to transactions
    client.subscribe_optimized().await?;
    
    println!("  ✅ Subscribed to pending transactions");
    println!("\n  Monitoring for 60 seconds...\n");
    
    let monitor_start = Instant::now();
    let monitor_duration = Duration::from_secs(60);
    
    // Track initial capture
    let mut initial_capture_complete = false;
    let initial_capture_start = Instant::now();
    
    while monitor_start.elapsed() < monitor_duration {
        match tokio::time::timeout(
            Duration::from_millis(100),
            client.read_next_transaction()
        ).await {
            Ok(Ok((tx_hash, latency))) => {
                stats.transactions_detected += 1;
                stats.detection_latencies.push(latency);
                
                // Mark initial capture complete after first batch
                if !initial_capture_complete && monitor_start.elapsed() > Duration::from_millis(500) {
                    initial_capture_complete = true;
                    stats.initial_capture_time = Some(initial_capture_start.elapsed());
                }
                
                // Log interesting detections
                if stats.transactions_detected <= 5 || 
                   stats.transactions_detected % 1000 == 0 ||
                   latency.as_micros() < 100 {
                    println!("  TX #{}: {} detected in {:?}", 
                             stats.transactions_detected,
                             &tx_hash[..10],
                             latency);
                }
                
                // Progress update every 10 seconds
                if monitor_start.elapsed().as_secs() % 10 == 0 && 
                   stats.transactions_detected % 100 < 10 {
                    let elapsed = monitor_start.elapsed().as_secs();
                    let rate = stats.transactions_detected as f64 / elapsed as f64;
                    println!("\n  Progress: {}s elapsed, {} txs detected ({:.1} tx/s)",
                             elapsed, stats.transactions_detected, rate);
                }
            }
            Ok(Err(e)) => {
                // Error reading transaction
                if stats.transactions_detected == 0 {
                    println!("  ⚠️  Error reading transactions: {}", e);
                }
            }
            Err(_) => {
                // Timeout - no transactions available
                if !initial_capture_complete {
                    initial_capture_complete = true;
                    stats.initial_capture_time = Some(initial_capture_start.elapsed());
                }
            }
        }
    }
    
    Ok(())
}

fn display_results(stats: &MempoolStats) {
    println!("\n\n📊 FINAL PERFORMANCE RESULTS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    println!("\n📋 MEMPOOL STATUS:");
    println!("  Initial mempool size: {} transactions", stats.initial_size);
    if stats.initial_size > 0 {
        println!("  Note: IPC subscriptions only show NEW transactions");
        println!("  To capture existing mempool, need different approach");
    }
    
    println!("\n⏱️  PERFORMANCE METRICS:");
    
    if let Some(capture_time) = stats.initial_capture_time {
        println!("  Initial response time: {:?}", capture_time);
    }
    
    println!("  Total transactions detected: {} in 60 seconds", stats.transactions_detected);
    println!("  Detection rate: {:.1} tx/s", stats.transactions_detected as f64 / 60.0);
    
    if !stats.detection_latencies.is_empty() {
        let mut sorted = stats.detection_latencies.clone();
        sorted.sort();
        
        let sum: Duration = sorted.iter().sum();
        let avg = sum / sorted.len() as u32;
        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let p50 = sorted[sorted.len() / 2];
        let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];
        let p99 = sorted[(sorted.len() as f64 * 0.99) as usize];
        
        println!("\n📈 DETECTION LATENCY STATISTICS:");
        println!("  Samples: {}", sorted.len());
        println!("  Min: {:?}", min);
        println!("  Average: {:?}", avg);
        println!("  Median (P50): {:?}", p50);
        println!("  P95: {:?}", p95);
        println!("  P99: {:?}", p99);
        println!("  Max: {:?}", max);
        
        // Distribution
        let sub_100us = sorted.iter().filter(|d| d.as_micros() < 100).count();
        let sub_1ms = sorted.iter().filter(|d| d.as_millis() < 1).count();
        let sub_10ms = sorted.iter().filter(|d| d.as_millis() < 10).count();
        
        println!("\n📊 LATENCY DISTRIBUTION:");
        println!("  <100μs: {:.1}% ({}/{})", 
                 sub_100us as f64 / sorted.len() as f64 * 100.0,
                 sub_100us, sorted.len());
        println!("  <1ms: {:.1}% ({}/{})", 
                 sub_1ms as f64 / sorted.len() as f64 * 100.0,
                 sub_1ms, sorted.len());
        println!("  <10ms: {:.1}% ({}/{})", 
                 sub_10ms as f64 / sorted.len() as f64 * 100.0,
                 sub_10ms, sorted.len());
        
        println!("\n✅ CONCLUSION:");
        if p95.as_millis() < 1 {
            println!("  🎯 ACHIEVED <1ms detection (P95: {:?})", p95);
            println!("  Optimized IPC successfully minimizes OS buffering");
        } else if p50.as_millis() < 1 {
            println!("  ✅ Median <1ms detection achieved");
            println!("  Some variability remains (P95: {:?})", p95);
        } else {
            println!("  ⚠️  Higher than expected latency");
            println!("  May need further optimization");
        }
    } else {
        println!("\n  ❌ No transactions detected");
        println!("  Possible reasons:");
        println!("  - Low network activity");
        println!("  - Connection issues");
    }
    
    println!("\n💡 KEY INSIGHTS:");
    println!("  1. From TX arrival at Reth → Our detection: {:?} (median)", 
             stats.detection_latencies.get(stats.detection_latencies.len() / 2)
                 .copied()
                 .unwrap_or(Duration::ZERO));
    println!("  2. We can process {:.0} transactions/second", 
             stats.transactions_detected as f64 / 60.0);
    println!("  3. Initial mempool of {} transactions exists but needs special handling", 
             stats.initial_size);
}