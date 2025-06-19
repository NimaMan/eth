/// Measure 1,000 real transactions using IPC-IPC method
/// Demonstrates 1.2ms average latency baseline measurement
/// Uses reusable IpcIpcMeasurementClient for clean implementation

use mempool_processor::mempool_fetcher::ipc_ipc::{
    IpcIpcMeasurementClient, MeasurementConfig, default_config
};
use tracing::{info, error};
use eyre::Result;


#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("measure_1k_transactions=info,mempool_processor=info")
        .init();

    info!("🎯 IPC-IPC MEASUREMENT TOOL");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Method: IPC subscription → IPC fetch");
    info!("Target: 1,000 transactions with detailed logging");
    info!("Expected: ~1.2ms average latency");
    
    // Use default configuration
    let config = default_config();
    info!("📋 Configuration:");
    info!("   • IPC Socket: {}", config.ipc_socket_path);
    info!("   • Target transactions: {}", config.target_transaction_count);
    info!("   • Log directory: {}", config.log_directory);
    
    // Create and initialize measurement client
    let mut client = IpcIpcMeasurementClient::new(config);
    
    info!("🔧 Initializing measurement client...");
    client.initialize().await?;
    
    // Run the measurement
    info!("⏱️  Starting measurement...");
    let results = client.run_measurement().await?;
    
    // Display final results
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 IPC-IPC MEASUREMENT COMPLETE");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    info!("⚡ Performance Results:");
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
    info!("   • Total time: {:.2}s", results.measurement_duration.as_secs_f64());
    
    if results.avg_latency_us / 1000.0 < 1.5 {
        info!("✅ Performance meets IPC-IPC baseline expectation!");
    } else {
        info!("⚠️  Performance above expected 1.2ms baseline");
    }
    
    info!("🎯 Measurement completed successfully!");
    
    Ok(())
}