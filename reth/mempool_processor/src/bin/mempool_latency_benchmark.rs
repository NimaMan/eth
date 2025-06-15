/// Mempool Latency Benchmark Tool
/// 
/// This tool measures the exact time it takes from a transaction's arrival
/// in the mempool until we detect it using our streaming methods.
///
/// Features:
/// - Real-time latency measurement
/// - Statistics with percentiles
/// - Comparison between WebSocket and DevP2P
/// - SLA compliance monitoring (<50ms target)
///
/// Usage:
///   cargo run --bin mempool_latency_benchmark

use mempool_processor::mempool_fetcher::{
    MempoolStreamer, StreamerConfig, StreamMode, TimestampedTransaction
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error};
use std::collections::VecDeque;
// use tokio::signal; // Not used in this implementation

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info,mempool_latency_benchmark=info")
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("🚀 Mempool Latency Benchmark Tool");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Target: <50ms detection latency");
    info!("SLA: 95% of transactions under target");
    
    // Configuration from environment
    let ws_url = std::env::var("ETH_RPC_WS").unwrap_or_else(|_| "ws://localhost:8546".to_string());
    let http_url = std::env::var("ETH_RPC_HTTP").unwrap_or_else(|_| "http://localhost:8545".to_string());
    
    info!("WebSocket: {}", ws_url);
    info!("HTTP: {}", http_url);
    
    // Test WebSocket streaming first
    info!("\n📡 Testing WebSocket Streaming Method...");
    let ws_results = run_latency_test(StreamMode::WebSocket, &ws_url, &http_url).await?;
    
    // Test DevP2P if available
    info!("\n🔗 Testing DevP2P Method...");
    let devp2p_results = match run_latency_test(StreamMode::DevP2P, &ws_url, &http_url).await {
        Ok(results) => Some(results),
        Err(e) => {
            warn!("DevP2P test failed: {}", e);
            None
        }
    };
    
    // Generate comprehensive report
    info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 LATENCY BENCHMARK RESULTS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // WebSocket Results
    print_test_results("WebSocket Streaming", &ws_results);
    
    // DevP2P Results (if available)
    if let Some(devp2p_results) = &devp2p_results {
        print_test_results("DevP2P Direct", devp2p_results);
        
        // Comparison
        info!("\n🆚 Performance Comparison:");
        let ws_avg = ws_results.avg_latency_ms;
        let devp2p_avg = devp2p_results.avg_latency_ms;
        let improvement = (ws_avg - devp2p_avg) / ws_avg * 100.0;
        
        info!("  WebSocket average: {:.2}ms", ws_avg);
        info!("  DevP2P average: {:.2}ms", devp2p_avg);
        info!("  DevP2P improvement: {:.1}% faster", improvement);
        
        if devp2p_avg < 10.0 {
            info!("  ✅ DevP2P meets <10ms target!");
        } else {
            warn!("  ⚠️  DevP2P above 10ms target");
        }
    }
    
    // SLA Analysis
    info!("\n🎯 SLA Analysis (Target: <50ms for 95% of transactions):");
    let ws_sla_compliance = calculate_sla_compliance(&ws_results, 50.0);
    info!("  WebSocket SLA compliance: {:.1}%", ws_sla_compliance);
    
    if let Some(devp2p_results) = &devp2p_results {
        let devp2p_sla_compliance = calculate_sla_compliance(devp2p_results, 50.0);
        info!("  DevP2P SLA compliance: {:.1}%", devp2p_sla_compliance);
        
        if devp2p_sla_compliance >= 95.0 {
            info!("  ✅ DevP2P meets SLA requirement!");
        } else {
            warn!("  ⚠️  DevP2P below SLA requirement");
        }
    }
    
    if ws_sla_compliance >= 95.0 {
        info!("  ✅ WebSocket meets SLA requirement!");
    } else {
        warn!("  ⚠️  WebSocket below SLA requirement");
    }
    
    // Recommendations
    info!("\n💡 Recommendations:");
    if ws_results.avg_latency_ms > 50.0 {
        info!("  ⚠️  High WebSocket latency detected:");
        info!("     - Ensure local Ethereum node for minimal network delay");
        info!("     - Check WebSocket endpoint performance");
        info!("     - Consider DevP2P implementation");
    } else {
        info!("  ✅ WebSocket latency within acceptable range");
    }
    
    if let Some(devp2p_results) = &devp2p_results {
        if devp2p_results.avg_latency_ms > 10.0 {
            info!("  ⚠️  DevP2P latency above optimal:");
            info!("     - Check IPC socket performance");
            info!("     - Verify local node configuration");
        } else {
            info!("  ✅ DevP2P achieving optimal latency");
        }
    } else {
        info!("  📝 Complete DevP2P implementation for <10ms latency");
    }
    
    // Save detailed results
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    save_benchmark_results(timestamp, &ws_results, devp2p_results.as_ref()).await?;
    
    info!("\n🎯 Benchmark completed successfully!");
    
    Ok(())
}

/// Test results for a specific streaming method
#[derive(Debug, Clone)]
struct TestResults {
    mode: StreamMode,
    duration: Duration,
    total_transactions: u64,
    avg_latency_ms: f64,
    min_latency_ms: f64,
    max_latency_ms: f64,
    p50_latency_ms: f64,
    p95_latency_ms: f64,
    p99_latency_ms: f64,
    transactions_per_second: f64,
    latency_samples: Vec<f64>,
}

/// Run a latency test for a specific streaming method
async fn run_latency_test(
    mode: StreamMode,
    ws_url: &str,
    http_url: &str,
) -> Result<TestResults, Box<dyn std::error::Error>> {
    info!("🔄 Initializing {} test...", format!("{:?}", mode));
    
    // Create configuration for the test
    let config = StreamerConfig {
        ws_url: Some(ws_url.to_string()),
        http_url: http_url.to_string(),
        devp2p_addr: Some("127.0.0.1:30303".to_string()),
        preferred_mode: mode.clone(),
        track_latency: true,
        initial_capture_duration: Duration::from_secs(10), // Shorter for testing
    };
    
    // Initialize streamer
    let streamer = MempoolStreamer::new(config).await?;
    
    // Start streaming
    streamer.start_streaming().await?;
    info!("✅ Streaming started, collecting latency data...");
    
    // Collect data for test duration
    let test_duration = Duration::from_secs(60); // 1 minute test
    let start_time = Instant::now();
    let mut latency_samples = Vec::new();
    let mut transaction_count = 0u64;
    
    // Track latency over time
    let mut latency_window = VecDeque::new();
    let window_size = 100; // Rolling window for real-time stats
    
    while start_time.elapsed() < test_duration {
        // Get new transactions
        let transactions = streamer.get_transactions(50).await?;
        
        for tx in transactions {
            transaction_count += 1;
            latency_samples.push(tx.latency_ms);
            
            // Update rolling window
            latency_window.push_back(tx.latency_ms);
            if latency_window.len() > window_size {
                latency_window.pop_front();
            }
            
            // Log high latency transactions
            if tx.latency_ms > 100.0 {
                warn!("High latency transaction: {:.2}ms", tx.latency_ms);
            }
            
            // Progress update every 100 transactions
            if transaction_count % 100 == 0 {
                let current_avg = latency_window.iter().sum::<f64>() / latency_window.len() as f64;
                let elapsed = start_time.elapsed();
                let tps = transaction_count as f64 / elapsed.as_secs_f64();
                
                info!("  Progress: {} txs, avg latency: {:.2}ms, rate: {:.1} TPS", 
                      transaction_count, current_avg, tps);
            }
        }
        
        // Small delay to prevent busy loop
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    let total_duration = start_time.elapsed();
    
    // Calculate statistics
    if latency_samples.is_empty() {
        return Err("No transactions collected during test".into());
    }
    
    let mut sorted_samples = latency_samples.clone();
    sorted_samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let total_transactions = transaction_count;
    let avg_latency_ms = latency_samples.iter().sum::<f64>() / latency_samples.len() as f64;
    let min_latency_ms = *sorted_samples.first().unwrap();
    let max_latency_ms = *sorted_samples.last().unwrap();
    let p50_latency_ms = sorted_samples[sorted_samples.len() / 2];
    let p95_latency_ms = sorted_samples[sorted_samples.len() * 95 / 100];
    let p99_latency_ms = sorted_samples[sorted_samples.len() * 99 / 100];
    let transactions_per_second = total_transactions as f64 / total_duration.as_secs_f64();
    
    Ok(TestResults {
        mode,
        duration: total_duration,
        total_transactions,
        avg_latency_ms,
        min_latency_ms,
        max_latency_ms,
        p50_latency_ms,
        p95_latency_ms,
        p99_latency_ms,
        transactions_per_second,
        latency_samples,
    })
}

/// Print formatted test results
fn print_test_results(method_name: &str, results: &TestResults) {
    info!("\n📊 {} Results:", method_name);
    info!("  Test duration: {:?}", results.duration);
    info!("  Transactions processed: {}", results.total_transactions);
    info!("  Transaction rate: {:.2} TPS", results.transactions_per_second);
    info!("");
    info!("  ⏱️  Latency Statistics:");
    info!("    Average: {:.2}ms", results.avg_latency_ms);
    info!("    Minimum: {:.2}ms", results.min_latency_ms);
    info!("    Maximum: {:.2}ms", results.max_latency_ms);
    info!("    P50 (median): {:.2}ms", results.p50_latency_ms);
    info!("    P95: {:.2}ms", results.p95_latency_ms);
    info!("    P99: {:.2}ms", results.p99_latency_ms);
    
    // Performance assessment
    let target_latency = match results.mode {
        StreamMode::WebSocket => 50.0,
        StreamMode::DevP2P => 10.0,
        _ => 50.0,
    };
    
    if results.avg_latency_ms < target_latency {
        info!("    ✅ PASSED: Average latency under {}ms target", target_latency);
    } else {
        warn!("    ❌ FAILED: Average latency above {}ms target", target_latency);
    }
}

/// Calculate SLA compliance percentage
fn calculate_sla_compliance(results: &TestResults, target_ms: f64) -> f64 {
    let under_target = results.latency_samples.iter()
        .filter(|&&latency| latency < target_ms)
        .count();
    
    (under_target as f64 / results.latency_samples.len() as f64) * 100.0
}

/// Save benchmark results to file
async fn save_benchmark_results(
    timestamp: u64,
    ws_results: &TestResults,
    devp2p_results: Option<&TestResults>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut results = serde_json::json!({
        "timestamp": timestamp,
        "benchmark_duration_sec": ws_results.duration.as_secs(),
        "websocket": {
            "total_transactions": ws_results.total_transactions,
            "avg_latency_ms": ws_results.avg_latency_ms,
            "min_latency_ms": ws_results.min_latency_ms,
            "max_latency_ms": ws_results.max_latency_ms,
            "p50_latency_ms": ws_results.p50_latency_ms,
            "p95_latency_ms": ws_results.p95_latency_ms,
            "p99_latency_ms": ws_results.p99_latency_ms,
            "transactions_per_second": ws_results.transactions_per_second,
            "sla_compliance_50ms": calculate_sla_compliance(ws_results, 50.0),
        }
    });
    
    if let Some(devp2p_results) = devp2p_results {
        results["devp2p"] = serde_json::json!({
            "total_transactions": devp2p_results.total_transactions,
            "avg_latency_ms": devp2p_results.avg_latency_ms,
            "min_latency_ms": devp2p_results.min_latency_ms,
            "max_latency_ms": devp2p_results.max_latency_ms,
            "p50_latency_ms": devp2p_results.p50_latency_ms,
            "p95_latency_ms": devp2p_results.p95_latency_ms,
            "p99_latency_ms": devp2p_results.p99_latency_ms,
            "transactions_per_second": devp2p_results.transactions_per_second,
            "sla_compliance_10ms": calculate_sla_compliance(devp2p_results, 10.0),
        });
    }
    
    let filename = format!("mempool_latency_benchmark_{}.json", timestamp);
    std::fs::write(&filename, serde_json::to_string_pretty(&results)?)?;
    info!("\n💾 Benchmark results saved to: {}", filename);
    
    Ok(())
}