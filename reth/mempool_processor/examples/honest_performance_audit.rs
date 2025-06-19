/// Honest Performance Audit - Measuring Real IPC Latency
/// This tool provides actual measurements instead of fabricated claims
/// 
/// Measures:
/// 1. IPC subscription latency (socket read to processing)
/// 2. Transaction fetching latency (request to response)
/// 3. End-to-end processing time
/// 4. Real throughput under load

use std::time::{Duration, Instant};
use mempool_processor::mempool_fetcher::ipc_socket::FullTxIpcClient;
use tracing::{info, warn};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("honest_performance_audit=info,mempool_processor=info")
        .init();

    info!("🔍 HONEST PERFORMANCE AUDIT");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("This audit will measure ACTUAL performance, not fabricated claims.");
    info!("All measurements will be reproducible and verifiable.");
    
    // Test 1: IPC Connection and Basic Latency
    info!("\n📊 Test 1: IPC Connection Latency");
    let connection_start = Instant::now();
    
    let ipc_client = match FullTxIpcClient::new(Some("/tmp/reth.ipc")) {
        Ok(client) => {
            let creation_time = connection_start.elapsed();
            info!("✅ IPC client created in {:?}", creation_time);
            client
        }
        Err(e) => {
            warn!("❌ Failed to create IPC client: {}", e);
            warn!("Make sure Reth is running with IPC enabled at /tmp/reth.ipc");
            return Ok(());
        }
    };
    
    // Test 2: Connection Establishment Time
    info!("\n📊 Test 2: IPC Socket Connection");
    let socket_start = Instant::now();
    
    match ipc_client.start_monitoring().await {
        Ok(()) => {
            let connection_time = socket_start.elapsed();
            info!("✅ IPC socket connected in {:?}", connection_time);
        }
        Err(e) => {
            warn!("❌ Failed to connect to IPC socket: {}", e);
            warn!("Ensure Reth node is running and IPC socket is accessible");
            return Ok(());
        }
    }
    
    // Wait for initial connection to stabilize
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Test 3: Real-time Transaction Processing Measurement
    info!("\n📊 Test 3: Real Transaction Processing Performance");
    info!("Measuring actual transaction latency for 60 seconds...");
    
    let measurement_duration = Duration::from_secs(60);
    let measurement_start = Instant::now();
    let mut total_transactions = 0;
    let mut latency_measurements = Vec::new();
    let mut processing_times = Vec::new();
    
    while measurement_start.elapsed() < measurement_duration {
        let fetch_start = Instant::now();
        
        // Get new transactions
        match ipc_client.get_full_transactions(20).await {
            Ok(transactions) if !transactions.is_empty() => {
                let fetch_time = fetch_start.elapsed();
                total_transactions += transactions.len();
                
                for tx in &transactions {
                    // Record the latency that was measured by the IPC client
                    latency_measurements.push(tx.latency_us);
                    
                    // Measure our processing time
                    let processing_time = fetch_time.as_micros() as u64;
                    processing_times.push(processing_time);
                }
                
                if total_transactions % 50 == 0 {
                    info!("  📈 Processed {} transactions so far...", total_transactions);
                }
            }
            Ok(_) => {
                // No transactions, small sleep
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(e) => {
                warn!("Error fetching transactions: {}", e);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
    
    let total_measurement_time = measurement_start.elapsed();
    
    // Test 4: Get IPC Client Statistics
    info!("\n📊 Test 4: IPC Client Internal Statistics");
    let ipc_stats = ipc_client.get_stats().await;
    let is_connected = ipc_client.is_connected().await;
    
    // Calculate real performance metrics
    info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 HONEST PERFORMANCE RESULTS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    info!("\n🔌 Connection Status:");
    info!("   • Connected: {}", is_connected);
    info!("   • Measurement duration: {:.2}s", total_measurement_time.as_secs_f64());
    
    info!("\n📦 Transaction Processing:");
    info!("   • Total transactions: {}", total_transactions);
    info!("   • Transactions per second: {:.2}", 
          total_transactions as f64 / total_measurement_time.as_secs_f64());
    
    if !latency_measurements.is_empty() {
        let avg_latency = latency_measurements.iter().sum::<u64>() as f64 / latency_measurements.len() as f64;
        let min_latency = latency_measurements.iter().min().unwrap_or(&0);
        let max_latency = latency_measurements.iter().max().unwrap_or(&0);
        
        let sub_1ms_count = latency_measurements.iter().filter(|&&l| l < 1000).count();
        let sub_10ms_count = latency_measurements.iter().filter(|&&l| l < 10000).count();
        
        info!("\n⚡ IPC Latency Analysis (Measured by IPC Client):");
        info!("   • Average latency: {:.1}μs ({:.3}ms)", avg_latency, avg_latency / 1000.0);
        info!("   • Min latency: {}μs ({:.3}ms)", min_latency, *min_latency as f64 / 1000.0);
        info!("   • Max latency: {}μs ({:.3}ms)", max_latency, *max_latency as f64 / 1000.0);
        info!("   • Sub-1ms transactions: {} ({:.1}%)", 
              sub_1ms_count, sub_1ms_count as f64 / latency_measurements.len() as f64 * 100.0);
        info!("   • Sub-10ms transactions: {} ({:.1}%)", 
              sub_10ms_count, sub_10ms_count as f64 / latency_measurements.len() as f64 * 100.0);
    } else {
        warn!("   • No latency measurements available");
    }
    
    if !processing_times.is_empty() {
        let avg_processing = processing_times.iter().sum::<u64>() as f64 / processing_times.len() as f64;
        let min_processing = processing_times.iter().min().unwrap_or(&0);
        let max_processing = processing_times.iter().max().unwrap_or(&0);
        
        info!("\n🔧 Our Processing Time:");
        info!("   • Average: {:.1}μs ({:.3}ms)", avg_processing, avg_processing / 1000.0);
        info!("   • Min: {}μs ({:.3}ms)", min_processing, *min_processing as f64 / 1000.0);
        info!("   • Max: {}μs ({:.3}ms)", max_processing, *max_processing as f64 / 1000.0);
    }
    
    info!("\n📊 IPC Client Internal Stats:");
    info!("   • Total transactions tracked: {}", ipc_stats.total_transactions);
    info!("   • Sub-1ms count: {}", ipc_stats.sub_1ms_count);
    info!("   • Sub-10ms count: {}", ipc_stats.sub_10ms_count);
    info!("   • Reported avg latency: {}μs", ipc_stats.avg_latency_us);
    
    // Throughput calculation
    if total_transactions > 0 && total_measurement_time.as_secs_f64() > 0.0 {
        let actual_tps = total_transactions as f64 / total_measurement_time.as_secs_f64();
        let avg_processing_ms = if !processing_times.is_empty() {
            processing_times.iter().sum::<u64>() as f64 / processing_times.len() as f64 / 1000.0
        } else {
            0.0
        };
        
        let theoretical_max_tps = if avg_processing_ms > 0.0 {
            1000.0 / avg_processing_ms  // Max TPS based on processing time
        } else {
            0.0
        };
        
        info!("\n🚀 Throughput Analysis:");
        info!("   • Measured TPS: {:.2}", actual_tps);
        info!("   • Theoretical max TPS: {:.2}", theoretical_max_tps);
        info!("   • Current utilization: {:.1}%", 
              if theoretical_max_tps > 0.0 { actual_tps / theoretical_max_tps * 100.0 } else { 0.0 });
    }
    
    info!("\n💡 Audit Conclusions:");
    info!("   ✅ All measurements are based on actual runtime data");
    info!("   ✅ Latency measurements use Rust's Instant::now() precision");
    info!("   ✅ Results are reproducible by running this audit again");
    info!("   ⚠️  Previous claims of '187,611 TPS' and '7.5ms mempool time' were UNSUPPORTED");
    
    if total_transactions == 0 {
        warn!("   ⚠️  No transactions processed during measurement period");
        warn!("   This could indicate:");
        warn!("   - Low network activity");
        warn!("   - IPC subscription not working");
        warn!("   - Node not fully synced");
    }
    
    info!("\n📋 Reproducibility:");
    info!("   • Run this audit multiple times to verify consistency");
    info!("   • Compare results during different network activity periods");
    info!("   • All timing uses std::time::Instant for accuracy");
    
    Ok(())
}