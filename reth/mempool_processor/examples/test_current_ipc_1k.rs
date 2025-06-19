/// Test Current IPC-IPC Implementation with 1,000 Transactions
/// 
/// Measures the performance of the current IPC-IPC implementation 
/// with exactly 1,000 transaction measurements.

use tracing::info;
use eyre::Result;

use mempool_processor::mempool_fetcher::ipc_ipc::{IpcIpcMeasurementClient, default_config};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("test_current_ipc_1k=info,mempool_processor=info")
        .init();

    info!("🚀 TESTING CURRENT IPC-IPC METHOD WITH 1,000 TRANSACTIONS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Method: IPC subscription + immediate IPC fetch");
    info!("Target: 1,000 new mempool transactions");
    info!("Socket: /tmp/reth.ipc");
    info!("");
    
    let mut config = default_config();
    config.target_transaction_count = 1000; // Exactly 1K transactions
    config.log_directory = "/home/nima/code/crypto/logs/mempool_fetch".to_string();

    let mut client = IpcIpcMeasurementClient::new(config);
    
    info!("🔧 Initializing measurement client...");
    client.initialize().await?;
    
    info!("⏱️  Starting 1K transaction measurement...");
    let start_time = std::time::Instant::now();
    let results = client.run_measurement().await?;
    let total_duration = start_time.elapsed();
    
    info!("");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 CURRENT IPC-IPC RESULTS (1,000 TRANSACTIONS)");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    info!("⚡ Performance Metrics:");
    info!("   • Method: IPC subscription → immediate IPC fetch");
    info!("   • Total transactions: {}", results.total_transactions);
    info!("   • Successful: {}", results.successful_transactions);
    info!("   • Average latency: {:.1}μs ({:.3}ms)", results.avg_latency_us, results.avg_latency_us / 1000.0);
    info!("   • Median latency: {}μs ({:.3}ms)", results.median_latency_us, results.median_latency_us as f64 / 1000.0);
    info!("   • Min latency: {}μs", results.min_latency_us);
    info!("   • Max latency: {}μs", results.max_latency_us);
    
    info!("📊 Distribution:");
    info!("   • Sub-1ms: {} ({:.1}%)", results.sub_1ms_count, results.sub_1ms_percentage);
    info!("   • Success rate: {:.1}%", results.success_rate_percentage);
    info!("   • Transactions/second: {:.2}", results.transactions_per_second);
    info!("   • Total measurement time: {:.2}s", total_duration.as_secs_f64());
    
    info!("");
    info!("🔍 Technical Details:");
    info!("   • Connection: Single Unix IPC socket (/tmp/reth.ipc)");
    info!("   • Subscription: eth_subscribe('newPendingTransactions')");
    info!("   • Fetch: Immediate eth_getTransactionByHash per notification");
    info!("   • Timing: From notification receipt → complete Transaction object");
    
    info!("");
    info!("📝 Key Performance Data:");
    info!("   • Average latency: {:.3}ms", results.avg_latency_us / 1000.0);
    info!("   • Sub-1ms percentage: {:.1}%", results.sub_1ms_percentage);
    info!("   • Throughput: {:.1} TPS", results.transactions_per_second);
    info!("   • Success rate: {:.1}%", results.success_rate_percentage);
    
    // Performance evaluation
    if results.avg_latency_us / 1000.0 < 1.0 {
        info!("   ✅ Excellent: Average latency under 1ms");
    } else if results.avg_latency_us / 1000.0 < 2.0 {
        info!("   ✅ Good: Average latency under 2ms");
    } else if results.avg_latency_us / 1000.0 < 5.0 {
        info!("   ⚠️  Fair: Average latency under 5ms");
    } else {
        info!("   ❌ Poor: Average latency over 5ms");
    }
    
    if results.sub_1ms_percentage > 60.0 {
        info!("   ✅ Excellent: >60% sub-1ms performance");
    } else if results.sub_1ms_percentage > 40.0 {
        info!("   ✅ Good: >40% sub-1ms performance");
    } else if results.sub_1ms_percentage > 20.0 {
        info!("   ⚠️  Fair: >20% sub-1ms performance");
    } else {
        info!("   ❌ Poor: <20% sub-1ms performance");
    }
    
    info!("");
    info!("📁 Data saved to: /home/nima/code/crypto/logs/mempool_fetch/");
    info!("✅ Current IPC-IPC 1K transaction test complete!");
    
    Ok(())
}