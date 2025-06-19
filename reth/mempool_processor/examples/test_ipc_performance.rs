/// Test IPC performance for fetching full transaction data
/// 
/// This example connects to the IPC socket and measures how quickly
/// we can get full transaction data without HTTP RPC overhead.

use std::time::{Duration, Instant};
use eyre::Result;
use tracing::{info, debug};

use mempool_processor::mempool_fetcher::ipc_socket::FullTxIpcClient;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(true)
        .with_env_filter("mempool_processor=debug")
        .init();
    
    info!("🚀 Testing IPC Performance for Full Transaction Data");
    
    // Configuration
    let ipc_path = "/tmp/reth.ipc";
    let test_transactions = 100;
    
    // Check if IPC socket exists
    if !std::path::Path::new(ipc_path).exists() {
        return Err(eyre::eyre!("IPC socket not found at {}. Make sure Reth is running with IPC enabled.", ipc_path));
    }
    
    info!("✅ IPC socket found at {}", ipc_path);
    
    // Connect to IPC
    let client = FullTxIpcClient::new(Some(ipc_path))?;
    info!("🔌 Starting IPC monitoring...");
    client.start_monitoring().await?;
    
    // Wait for connection to establish
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    if !client.is_connected().await {
        return Err(eyre::eyre!("Failed to establish IPC connection"));
    }
    
    info!("✅ IPC connection established");
    info!("📊 Collecting {} transactions...", test_transactions);
    
    let mut collected = 0;
    let start_time = Instant::now();
    let mut first_tx_time = None;
    let mut transaction_times = Vec::new();
    
    while collected < test_transactions {
        let txs = client.get_full_transactions(10).await?;
        
        for tx in txs {
            if first_tx_time.is_none() {
                first_tx_time = Some(Instant::now());
                info!("🎯 First transaction received after {:.1}s", start_time.elapsed().as_secs_f64());
            }
            
            collected += 1;
            transaction_times.push(tx.latency_us);
            
            // Log first few transactions
            if collected <= 5 {
                info!("  TX {}: {} ({}μs = {:.1}ms)", 
                      collected,
                      &tx.hash[..10],
                      tx.latency_us,
                      tx.latency_us as f64 / 1000.0);
                
                // Show some transaction details
                debug!("    From: {:?}", tx.transaction.from);
                debug!("    To: {:?}", tx.transaction.to);
                debug!("    Value: {:?}", tx.transaction.value);
                debug!("    Gas: {:?}", tx.transaction.gas);
            }
            
            if collected >= test_transactions {
                break;
            }
        }
        
        if txs.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    
    let total_time = start_time.elapsed();
    
    // Calculate statistics
    transaction_times.sort();
    let avg_latency = transaction_times.iter().sum::<u64>() / transaction_times.len() as u64;
    let min_latency = transaction_times[0];
    let max_latency = transaction_times[transaction_times.len() - 1];
    let p50 = transaction_times[transaction_times.len() / 2];
    let p95 = transaction_times[(transaction_times.len() as f64 * 0.95) as usize];
    let p99 = transaction_times[(transaction_times.len() as f64 * 0.99) as usize];
    
    // Count sub-millisecond transactions
    let sub_1ms = transaction_times.iter().filter(|&&t| t < 1000).count();
    let sub_10ms = transaction_times.iter().filter(|&&t| t < 10000).count();
    let sub_100ms = transaction_times.iter().filter(|&&t| t < 100000).count();
    
    info!("\n📊 Performance Results:");
    info!("  Total transactions: {}", collected);
    info!("  Total time: {:.1}s", total_time.as_secs_f64());
    info!("  Transactions/second: {:.1}", collected as f64 / total_time.as_secs_f64());
    
    info!("\n⏱️  Latency Statistics (includes full transaction data):");
    info!("  Average: {}μs ({:.1}ms)", avg_latency, avg_latency as f64 / 1000.0);
    info!("  Min: {}μs ({:.1}ms)", min_latency, min_latency as f64 / 1000.0);
    info!("  P50: {}μs ({:.1}ms)", p50, p50 as f64 / 1000.0);
    info!("  P95: {}μs ({:.1}ms)", p95, p95 as f64 / 1000.0);
    info!("  P99: {}μs ({:.1}ms)", p99, p99 as f64 / 1000.0);
    info!("  Max: {}μs ({:.1}ms)", max_latency, max_latency as f64 / 1000.0);
    
    info!("\n📈 Latency Distribution:");
    info!("  <1ms: {} ({:.1}%)", sub_1ms, sub_1ms as f64 / collected as f64 * 100.0);
    info!("  <10ms: {} ({:.1}%)", sub_10ms, sub_10ms as f64 / collected as f64 * 100.0);
    info!("  <100ms: {} ({:.1}%)", sub_100ms, sub_100ms as f64 / collected as f64 * 100.0);
    
    // Get overall stats from client
    let stats = client.get_stats().await;
    info!("\n📊 Overall Client Statistics:");
    info!("  Total transactions seen: {}", stats.total_transactions);
    info!("  Average latency: {}μs ({:.1}ms)", stats.avg_latency_us, stats.avg_latency_us as f64 / 1000.0);
    
    info!("\n✅ Test complete!");
    
    // Compare with WebSocket+HTTP expected performance
    info!("\n🔍 Comparison:");
    info!("  WebSocket+HTTP expected: 50-200ms per transaction");
    info!("  IPC actual: {:.1}ms average", avg_latency as f64 / 1000.0);
    info!("  Improvement: ~{:.0}x faster", 100.0 / (avg_latency as f64 / 1000.0));
    
    Ok(())
}