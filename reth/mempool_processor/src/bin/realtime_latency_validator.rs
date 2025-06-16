/// Real-time Latency Validator for <10ms Requirement
/// 
/// This tool validates that our mempool fetching meets the strict <10ms
/// real-time latency requirement. It tests both DevP2P and optimized WebSocket
/// approaches and provides pass/fail validation.
///
/// Usage:
///   cargo run --bin realtime_latency_validator

use mempool_processor::mempool_fetcher::{
    MempoolStreamer, StreamerConfig, StreamMode, TimestampedTransaction
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info, warn, error, debug};
use std::collections::VecDeque;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info,realtime_latency_validator=info")
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("⚡ Real-time Latency Validator (<10ms Requirement)");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🎯 TARGET: <10ms detection latency for real-time transactions");
    info!("📏 SLA: 95% of transactions under 10ms");
    
    let ws_url = std::env::var("ETH_RPC_WS").unwrap_or_else(|_| "ws://localhost:8546".to_string());
    let http_url = std::env::var("ETH_RPC_HTTP").unwrap_or_else(|_| "http://localhost:8545".to_string());
    
    info!("WebSocket: {}", ws_url);
    info!("HTTP: {}", http_url);
    
    // Test 1: DevP2P Method (Primary target for <10ms)
    info!("\n🔗 Test 1: DevP2P Direct Connection");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let devp2p_result = test_devp2p_latency(&ws_url, &http_url).await;
    
    // Test 2: Optimized WebSocket (Interim solution)
    info!("\n📡 Test 2: Optimized WebSocket Streaming");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let websocket_result = test_websocket_latency(&ws_url, &http_url).await;
    
    // Validation Results
    info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🎯 REAL-TIME LATENCY VALIDATION RESULTS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let mut passed_tests = 0;
    let total_tests = 2;
    
    // DevP2P Results
    match devp2p_result {
        Ok(result) => {
            info!("\n🔗 DevP2P Test Results:");
            print_validation_result("DevP2P", &result, 10.0);
            if result.meets_sla(10.0, 95.0) {
                passed_tests += 1;
            }
        }
        Err(e) => {
            warn!("❌ DevP2P test failed: {}", e);
            info!("   This is expected if DevP2P implementation is not complete");
        }
    }
    
    // WebSocket Results
    match websocket_result {
        Ok(result) => {
            info!("\n📡 WebSocket Test Results:");
            print_validation_result("WebSocket", &result, 10.0);
            // For WebSocket, we use a more lenient target since it's interim
            if result.meets_sla(25.0, 95.0) {
                passed_tests += 1;
            }
        }
        Err(e) => {
            error!("❌ WebSocket test failed: {}", e);
        }
    }
    
    // Overall Assessment
    info!("\n🏆 Overall Assessment:");
    info!("  Tests passed: {}/{}", passed_tests, total_tests);
    
    if devp2p_result.as_ref().map_or(false, |r| r.meets_sla(10.0, 95.0)) {
        info!("  ✅ PASSED: DevP2P achieves <10ms real-time requirement!");
    } else if websocket_result.as_ref().map_or(false, |r| r.avg_latency_ms < 25.0) {
        info!("  ⚠️  PARTIAL: WebSocket provides interim solution (~{}ms average)", 
              websocket_result.as_ref().unwrap().avg_latency_ms);
        info!("     Complete DevP2P implementation needed for <10ms target");
    } else {
        warn!("  ❌ FAILED: Neither method meets real-time requirements");
    }
    
    // Recommendations
    info!("\n💡 Recommendations:");
    
    if devp2p_result.is_err() {
        info!("  🔧 Complete DevP2P implementation for <10ms latency");
        info!("     - Currently: Protocol framework implemented");
        info!("     - Needed: Full message handling and testing");
    }
    
    if let Ok(ws_result) = &websocket_result {
        if ws_result.avg_latency_ms > 10.0 {
            info!("  ⚡ WebSocket optimizations:");
            info!("     - Use newPendingTransactionsFull if available");
            info!("     - Reduce polling intervals");
            info!("     - Consider direct WebSocket frame parsing");
        }
    }
    
    info!("\n🎯 Next Steps for <10ms Compliance:");
    info!("  1. Complete DevP2P protocol testing");
    info!("  2. Validate on local Ethereum node");
    info!("  3. Measure against live transaction flow");
    info!("  4. Optimize based on real-world performance");
    
    // Save validation results
    save_validation_results(&devp2p_result, &websocket_result).await?;
    
    Ok(())
}

/// Test DevP2P latency performance
async fn test_devp2p_latency(
    ws_url: &str,
    http_url: &str,
) -> Result<LatencyTestResult, Box<dyn std::error::Error>> {
    info!("🔄 Testing DevP2P method...");
    
    let config = StreamerConfig {
        ws_url: Some(ws_url.to_string()),
        http_url: http_url.to_string(),
        devp2p_addr: Some("127.0.0.1:30303".to_string()),
        preferred_mode: StreamMode::DevP2P,
        track_latency: true,
        initial_capture_duration: Duration::from_secs(5),
        ..Default::default()
    };
    
    let streamer = MempoolStreamer::new(config).await?;
    
    // Start streaming
    streamer.start_streaming().await?;
    
    // Test for 30 seconds to get good data
    let test_duration = Duration::from_secs(30);
    let mut latency_samples = Vec::new();
    let start_time = Instant::now();
    
    info!("📊 Collecting DevP2P latency data for {} seconds...", test_duration.as_secs());
    
    while start_time.elapsed() < test_duration {
        let transactions = streamer.get_transactions(10).await?;
        
        for tx in transactions {
            latency_samples.push(tx.latency_ms);
            
            // Log very low latency achievements
            if tx.latency_ms < 10.0 {
                debug!("⚡ Low latency achieved: {:.2}ms", tx.latency_ms);
            }
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    if latency_samples.is_empty() {
        return Err("No transactions received during DevP2P test".into());
    }
    
    Ok(LatencyTestResult::from_samples("DevP2P", latency_samples))
}

/// Test optimized WebSocket latency performance
async fn test_websocket_latency(
    ws_url: &str,
    http_url: &str,
) -> Result<LatencyTestResult, Box<dyn std::error::Error>> {
    info!("🔄 Testing optimized WebSocket method...");
    
    let config = StreamerConfig {
        ws_url: Some(ws_url.to_string()),
        http_url: http_url.to_string(),
        preferred_mode: StreamMode::WebSocket,
        track_latency: true,
        initial_capture_duration: Duration::from_secs(5),
        ..Default::default()
    };
    
    let streamer = MempoolStreamer::new(config).await?;
    
    // Start streaming
    streamer.start_streaming().await?;
    
    // Test for 30 seconds
    let test_duration = Duration::from_secs(30);
    let mut latency_samples = Vec::new();
    let start_time = Instant::now();
    
    info!("📊 Collecting WebSocket latency data for {} seconds...", test_duration.as_secs());
    
    while start_time.elapsed() < test_duration {
        let transactions = streamer.get_transactions(10).await?;
        
        for tx in transactions {
            latency_samples.push(tx.latency_ms);
            
            // Log exceptional performance
            if tx.latency_ms < 15.0 {
                debug!("⚡ Good WebSocket latency: {:.2}ms", tx.latency_ms);
            }
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    if latency_samples.is_empty() {
        return Err("No transactions received during WebSocket test".into());
    }
    
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
        let compliance = (self.samples_under_target(target_ms) as f64 / self.total_samples as f64) * 100.0;
        compliance >= percentage
    }
    
    fn samples_under_target(&self, target_ms: f64) -> usize {
        if target_ms <= 10.0 {
            self.samples_under_10ms
        } else {
            self.samples_under_25ms
        }
    }
}

/// Print validation results
fn print_validation_result(method: &str, result: &LatencyTestResult, target_ms: f64) {
    info!("  Method: {}", method);
    info!("  Samples: {}", result.total_samples);
    info!("  Average: {:.2}ms", result.avg_latency_ms);
    info!("  P95: {:.2}ms", result.p95_latency_ms);
    info!("  P99: {:.2}ms", result.p99_latency_ms);
    info!("  Min: {:.2}ms, Max: {:.2}ms", result.min_latency_ms, result.max_latency_ms);
    
    let compliance = (result.samples_under_target(target_ms) as f64 / result.total_samples as f64) * 100.0;
    info!("  Under {}ms: {:.1}%", target_ms, compliance);
    
    if result.meets_sla(target_ms, 95.0) {
        info!("  ✅ PASSED: Meets <{}ms SLA (95%)", target_ms);
    } else {
        warn!("  ❌ FAILED: Does not meet <{}ms SLA (95%)", target_ms);
    }
}

/// Save validation results to file
async fn save_validation_results(
    devp2p_result: &Result<LatencyTestResult, Box<dyn std::error::Error>>,
    websocket_result: &Result<LatencyTestResult, Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    
    let mut results = serde_json::json!({
        "timestamp": timestamp,
        "validation_target": "<10ms real-time latency",
        "sla_requirement": "95% under target"
    });
    
    if let Ok(devp2p) = devp2p_result {
        results["devp2p"] = serde_json::json!({
            "method": devp2p.method,
            "samples": devp2p.total_samples,
            "avg_latency_ms": devp2p.avg_latency_ms,
            "p95_latency_ms": devp2p.p95_latency_ms,
            "p99_latency_ms": devp2p.p99_latency_ms,
            "under_10ms_percent": (devp2p.samples_under_10ms as f64 / devp2p.total_samples as f64) * 100.0,
            "meets_sla": devp2p.meets_sla(10.0, 95.0),
            "status": if devp2p.meets_sla(10.0, 95.0) { "PASSED" } else { "FAILED" }
        });
    } else {
        results["devp2p"] = serde_json::json!({
            "status": "NOT_AVAILABLE",
            "reason": "Implementation not complete"
        });
    }
    
    if let Ok(websocket) = websocket_result {
        results["websocket"] = serde_json::json!({
            "method": websocket.method,
            "samples": websocket.total_samples,
            "avg_latency_ms": websocket.avg_latency_ms,
            "p95_latency_ms": websocket.p95_latency_ms,
            "p99_latency_ms": websocket.p99_latency_ms,
            "under_10ms_percent": (websocket.samples_under_10ms as f64 / websocket.total_samples as f64) * 100.0,
            "under_25ms_percent": (websocket.samples_under_25ms as f64 / websocket.total_samples as f64) * 100.0,
            "meets_10ms_sla": websocket.meets_sla(10.0, 95.0),
            "meets_25ms_sla": websocket.meets_sla(25.0, 95.0),
            "status": if websocket.meets_sla(10.0, 95.0) { "PASSED" } else { "INTERIM" }
        });
    }
    
    let filename = format!("realtime_latency_validation_{}.json", timestamp);
    std::fs::write(&filename, serde_json::to_string_pretty(&results)?)?;
    info!("\n💾 Validation results saved to: {}", filename);
    
    Ok(())
}