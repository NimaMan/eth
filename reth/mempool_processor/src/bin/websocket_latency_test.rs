/// Simple WebSocket Latency Test
/// Tests WebSocket streaming performance for <10ms requirement validation

use mempool_processor::mempool_fetcher::{WebSocketClient, TransactionView};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info,websocket_latency_test=info")
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("⚡ WebSocket Latency Test for <10ms Real-time Requirement");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let ws_url = std::env::var("ETH_RPC_WS").unwrap_or_else(|_| "ws://localhost:8546".to_string());
    let http_url = std::env::var("ETH_RPC_HTTP").unwrap_or_else(|_| "http://localhost:8545".to_string());
    
    info!("WebSocket: {}", ws_url);
    info!("HTTP: {}", http_url);
    
    // Test WebSocket streaming latency
    let test_result = test_websocket_latency(&ws_url, &http_url).await;
    
    match test_result {
        Ok(result) => {
            info!("🎯 WEBSOCKET LATENCY TEST RESULTS");
            info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            print_test_results(&result);
            
            // Save results
            save_test_results(&result).await?;
        }
        Err(e) => {
            error!("❌ WebSocket test failed: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

/// Test WebSocket latency performance
async fn test_websocket_latency(
    ws_url: &str,
    http_url: &str,
) -> Result<LatencyTestResult, Box<dyn std::error::Error>> {
    info!("🔄 Initializing WebSocket fetcher...");
    
    // Create WebSocket fetcher
    let fetcher = WebSocketClient::new(ws_url, http_url).await?;
    
    // Start full capture
    fetcher.start_full_capture().await?;
    
    // Test for 60 seconds to get good data
    let test_duration = Duration::from_secs(60);
    let mut latency_samples = Vec::new();
    let start_time = Instant::now();
    
    info!("📊 Collecting WebSocket latency data for {} seconds...", test_duration.as_secs());
    
    let mut total_transactions = 0;
    while start_time.elapsed() < test_duration {
        let transactions = fetcher.get_new_transactions(50).await?;
        
        if !transactions.is_empty() {
            total_transactions += transactions.len();
            for tx in transactions {
                latency_samples.push(tx.latency_ms);
                
                // Log excellent performance
                if tx.latency_ms < 10.0 {
                    info!("⚡ Excellent latency: {:.2}ms", tx.latency_ms);
                }
            }
            
            // Progress update
            if total_transactions % 100 == 0 && total_transactions > 0 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let tps = total_transactions as f64 / elapsed;
                info!("📈 Progress: {} transactions ({:.1} TPS)", total_transactions, tps);
            }
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    if latency_samples.is_empty() {
        return Err("No transactions received during test".into());
    }
    
    info!("✅ Test completed: {} transactions processed", total_transactions);
    
    Ok(LatencyTestResult::from_samples("WebSocket", latency_samples))
}

/// Latency test results
#[derive(Debug, Clone)]
struct LatencyTestResult {
    method: String,
    total_samples: usize,
    avg_latency_ms: f64,
    min_latency_ms: f64,
    max_latency_ms: f64,
    p50_latency_ms: f64,
    p95_latency_ms: f64,
    p99_latency_ms: f64,
    samples_under_10ms: usize,
    samples_under_25ms: usize,
}

impl LatencyTestResult {
    fn from_samples(method: &str, mut samples: Vec<f64>) -> Self {
        samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let total_samples = samples.len();
        let avg_latency_ms = samples.iter().sum::<f64>() / total_samples as f64;
        let min_latency_ms = samples[0];
        let max_latency_ms = samples[total_samples - 1];
        let p50_latency_ms = samples[total_samples / 2];
        let p95_latency_ms = samples[total_samples * 95 / 100];
        let p99_latency_ms = samples[total_samples * 99 / 100];
        
        let samples_under_10ms = samples.iter().filter(|&&x| x < 10.0).count();
        let samples_under_25ms = samples.iter().filter(|&&x| x < 25.0).count();
        
        Self {
            method: method.to_string(),
            total_samples,
            avg_latency_ms,
            min_latency_ms,
            max_latency_ms,
            p50_latency_ms,
            p95_latency_ms,
            p99_latency_ms,
            samples_under_10ms,
            samples_under_25ms,
        }
    }
    
    fn meets_sla(&self, target_ms: f64, percentage: f64) -> bool {
        let under_target = if target_ms <= 10.0 {
            self.samples_under_10ms
        } else {
            self.samples_under_25ms
        };
        let compliance = (under_target as f64 / self.total_samples as f64) * 100.0;
        compliance >= percentage
    }
}

/// Print test results
fn print_test_results(result: &LatencyTestResult) {
    info!("  Method: {}", result.method);
    info!("  Samples: {}", result.total_samples);
    info!("  Average: {:.2}ms", result.avg_latency_ms);
    info!("  P50: {:.2}ms", result.p50_latency_ms);
    info!("  P95: {:.2}ms", result.p95_latency_ms);
    info!("  P99: {:.2}ms", result.p99_latency_ms);
    info!("  Min: {:.2}ms, Max: {:.2}ms", result.min_latency_ms, result.max_latency_ms);
    
    let under_10ms_percent = (result.samples_under_10ms as f64 / result.total_samples as f64) * 100.0;
    let under_25ms_percent = (result.samples_under_25ms as f64 / result.total_samples as f64) * 100.0;
    
    info!("  Under 10ms: {:.1}% ({} transactions)", under_10ms_percent, result.samples_under_10ms);
    info!("  Under 25ms: {:.1}% ({} transactions)", under_25ms_percent, result.samples_under_25ms);
    
    // SLA Assessment
    if result.meets_sla(10.0, 95.0) {
        info!("  ✅ PASSED: Meets <10ms SLA (95% compliance)");
    } else if result.meets_sla(25.0, 95.0) {
        warn!("  ⚠️  PARTIAL: Meets <25ms but not <10ms target");
    } else {
        warn!("  ❌ FAILED: Does not meet latency requirements");
    }
    
    // Performance recommendations
    if result.avg_latency_ms > 10.0 {
        info!("💡 Recommendations:");
        if result.avg_latency_ms > 50.0 {
            info!("  - High latency detected - investigate network/RPC performance");
        }
        if under_10ms_percent < 50.0 {
            info!("  - Consider DevP2P implementation for <10ms target");
        }
        info!("  - Optimize WebSocket buffer sizes and polling intervals");
    }
}

/// Save test results to file
async fn save_test_results(
    result: &LatencyTestResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    
    let results_json = serde_json::json!({
        "timestamp": timestamp,
        "test_type": "websocket_latency_validation",
        "target": "<10ms real-time latency",
        "websocket": {
            "method": result.method,
            "samples": result.total_samples,
            "avg_latency_ms": result.avg_latency_ms,
            "p50_latency_ms": result.p50_latency_ms,
            "p95_latency_ms": result.p95_latency_ms,
            "p99_latency_ms": result.p99_latency_ms,
            "min_latency_ms": result.min_latency_ms,
            "max_latency_ms": result.max_latency_ms,
            "under_10ms_percent": (result.samples_under_10ms as f64 / result.total_samples as f64) * 100.0,
            "under_25ms_percent": (result.samples_under_25ms as f64 / result.total_samples as f64) * 100.0,
            "meets_10ms_sla": result.meets_sla(10.0, 95.0),
            "meets_25ms_sla": result.meets_sla(25.0, 95.0),
            "status": if result.meets_sla(10.0, 95.0) { "PASSED" } else { "NEEDS_IMPROVEMENT" }
        }
    });
    
    let filename = format!("websocket_latency_test_{}.json", timestamp);
    std::fs::write(&filename, serde_json::to_string_pretty(&results_json)?)?;
    info!("💾 Test results saved to: {}", filename);
    
    Ok(())
}