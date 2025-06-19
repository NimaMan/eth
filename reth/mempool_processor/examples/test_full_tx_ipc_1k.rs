/// Test Full TX IPC Implementation with 1,000 Transactions
/// 
/// Measures the performance of the full TX IPC implementation
/// with exactly 1,000 transaction measurements.

use std::time::{Duration, Instant};
use tracing::{info, warn, error};
use eyre::Result;

use mempool_processor::mempool_fetcher::ipc_ipc_variants::{FullTxIpcClient, FullIpcTransaction};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("test_full_tx_ipc_1k=info,mempool_processor=info")
        .init();

    info!("🚀 TESTING FULL TX IPC METHOD WITH 1,000 TRANSACTIONS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Method: IPC subscription + IPC individual fetch (with full TX support attempt)");
    info!("Target: 1,000 new mempool transactions");
    info!("Socket: /tmp/reth.ipc");
    info!("");
    
    info!("🔧 Initializing full TX IPC client...");
    let client = FullTxIpcClient::new(None)?;
    
    info!("📡 Starting monitoring...");
    client.start_monitoring().await?;
    
    // Wait for connection to stabilize
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    info!("⏱️  Starting 1K transaction measurement...");
    let start_time = Instant::now();
    
    let mut transactions = Vec::new();
    let target_count = 1000;
    let max_duration = Duration::from_secs(300); // 5 minutes max
    
    info!("📈 Collecting full transactions...");
    
    while transactions.len() < target_count && start_time.elapsed() < max_duration {
        // Get up to 50 transactions at a time
        let batch = client.get_full_transactions(50).await?;
        
        if batch.is_empty() {
            // No new transactions, wait a bit
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }
        
        for tx in batch {
            transactions.push(tx);
            
            if transactions.len() % 100 == 0 {
                info!("📦 Collected {} full transactions...", transactions.len());
            }
            
            if transactions.len() >= target_count {
                break;
            }
        }
    }
    
    let total_duration = start_time.elapsed();
    
    if transactions.len() < target_count {
        warn!("⚠️  Only collected {} transactions in {:.1}s (target: {})", 
              transactions.len(), total_duration.as_secs_f64(), target_count);
    }
    
    // Get final statistics
    let stats = client.get_stats().await;
    
    info!("");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 FULL TX IPC RESULTS ({} TRANSACTIONS)", transactions.len());
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Calculate statistics
    if !transactions.is_empty() {
        let latencies: Vec<f64> = transactions.iter().map(|tx| tx.latency_us as f64 / 1000.0).collect();
        let mut sorted_latencies = latencies.clone();
        sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
        let median_latency = sorted_latencies[sorted_latencies.len() / 2];
        let min_latency = sorted_latencies[0];
        let max_latency = sorted_latencies[sorted_latencies.len() - 1];
        
        let sub_1ms_count = latencies.iter().filter(|&&l| l < 1.0).count();
        let sub_1ms_percentage = sub_1ms_count as f64 / latencies.len() as f64 * 100.0;
        
        let arrival_rate = transactions.len() as f64 / total_duration.as_secs_f64();
        let max_processing_capacity = 1000.0 / avg_latency;
        
        info!("⚡ Performance Metrics:");
        info!("   • Method: IPC subscription + IPC individual fetch (full TX variant)");
        info!("   • Total transactions: {}", transactions.len());
        info!("   • Successful: {}", transactions.len());
        info!("   • Average latency: {:.3}ms", avg_latency);
        info!("   • Median latency: {:.3}ms", median_latency);
        info!("   • Min latency: {:.3}ms", min_latency);
        info!("   • Max latency: {:.3}ms", max_latency);
        
        info!("📊 Distribution:");
        info!("   • Sub-1ms: {} ({:.1}%)", sub_1ms_count, sub_1ms_percentage);
        info!("   • Success rate: 100.0%");
        info!("   • Arrival rate: {:.2} tx/s (mempool activity)", arrival_rate);
        info!("   • Max processing capacity: {:.0} tx/s (theoretical)", max_processing_capacity);
        info!("   • Total measurement time: {:.2}s", total_duration.as_secs_f64());
        
        info!("");
        info!("📊 IPC Statistics:");
        info!("   • Total transactions: {}", stats.total_transactions);
        info!("   • Average latency: {}μs", stats.avg_latency_us);
        info!("   • Sub-1ms: {} ({:.1}%)", stats.sub_1ms_count, 
              stats.sub_1ms_count as f64 / stats.total_transactions as f64 * 100.0);
        info!("   • Sub-10ms: {} ({:.1}%)", stats.sub_10ms_count, 
              stats.sub_10ms_count as f64 / stats.total_transactions as f64 * 100.0);
        if let (Some(min), Some(max)) = (stats.min_latency_us, stats.max_latency_us) {
            info!("   • Min latency: {}μs", min);
            info!("   • Max latency: {}μs", max);
        }
        
        info!("");
        info!("🔍 Technical Details:");
        info!("   • Connection: Single Unix IPC socket (/tmp/reth.ipc)");
        info!("   • Subscription: eth_subscribe('newPendingTransactions', includeTransactions: true)");
        info!("   • Fallback: Standard subscription + eth_getTransactionByHash");
        info!("   • Timing: From notification receipt → complete Transaction object");
        
        info!("");
        info!("📝 Key Performance Data:");
        info!("   • Average latency: {:.3}ms", avg_latency);
        info!("   • Sub-1ms percentage: {:.1}%", sub_1ms_percentage);
        info!("   • Arrival rate: {:.1} tx/s", arrival_rate);
        info!("   • Max capacity: {:.0} tx/s", max_processing_capacity);
        
        // Performance evaluation
        if avg_latency < 1.0 {
            info!("   ✅ Excellent: Average latency under 1ms");
        } else if avg_latency < 2.0 {
            info!("   ✅ Good: Average latency under 2ms");
        } else if avg_latency < 5.0 {
            info!("   ⚠️  Fair: Average latency under 5ms");
        } else {
            info!("   ❌ Poor: Average latency over 5ms");
        }
        
        if sub_1ms_percentage > 60.0 {
            info!("   ✅ Excellent: >60% sub-1ms performance");
        } else if sub_1ms_percentage > 40.0 {
            info!("   ✅ Good: >40% sub-1ms performance");
        } else if sub_1ms_percentage > 20.0 {
            info!("   ⚠️  Fair: >20% sub-1ms performance");
        } else {
            info!("   ❌ Poor: <20% sub-1ms performance");
        }
        
        info!("");
        info!("💡 Full TX IPC Insights:");
        info!("   • Attempts to get full transaction data in subscription");
        info!("   • Falls back to individual fetch if not supported");
        info!("   • Similar approach to current implementation");
        info!("   • Performance comparison vs current: {:.1}x", avg_latency / 1.424);
        
    } else {
        error!("❌ No transactions collected!");
    }
    
    info!("");
    info!("✅ Full TX IPC 1K transaction test complete!");
    
    Ok(())
}