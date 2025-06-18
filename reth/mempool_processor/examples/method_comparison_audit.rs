/// Method Comparison Audit - IPC vs RPC vs WebSocket
/// This provides side-by-side comparison of different transaction fetching methods
/// to verify which actually provides the lowest latency

use std::time::{Duration, Instant};
use mempool_processor::mempool_fetcher::ipc_socket::FullTxIpcClient;
use ethers::providers::{Provider, Http, Ws, Middleware};
use ethers::types::H256;
use tracing::{info, warn, error};
use eyre::Result;
use std::collections::VecDeque;

struct LatencyMeasurement {
    method: String,
    latency_us: u64,
    success: bool,
    timestamp: Instant,
}

struct PerformanceStats {
    measurements: VecDeque<LatencyMeasurement>,
    method_name: String,
}

impl PerformanceStats {
    fn new(method_name: String) -> Self {
        Self {
            measurements: VecDeque::new(),
            method_name,
        }
    }
    
    fn add_measurement(&mut self, latency_us: u64, success: bool) {
        self.measurements.push_back(LatencyMeasurement {
            method: self.method_name.clone(),
            latency_us,
            success,
            timestamp: Instant::now(),
        });
        
        // Keep only last 1000 measurements
        if self.measurements.len() > 1000 {
            self.measurements.pop_front();
        }
    }
    
    fn get_stats(&self) -> (f64, u64, u64, usize, usize) {
        let successful: Vec<_> = self.measurements.iter()
            .filter(|m| m.success)
            .collect();
            
        if successful.is_empty() {
            return (0.0, 0, 0, 0, self.measurements.len());
        }
        
        let latencies: Vec<u64> = successful.iter().map(|m| m.latency_us).collect();
        let avg = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
        let min = *latencies.iter().min().unwrap_or(&0);
        let max = *latencies.iter().max().unwrap_or(&0);
        
        (avg, min, max, successful.len(), self.measurements.len())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("method_comparison_audit=info,mempool_processor=warn")
        .init();

    info!("🔍 METHOD COMPARISON AUDIT");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Comparing IPC vs HTTP RPC vs WebSocket transaction fetching");
    
    let mut ipc_stats = PerformanceStats::new("IPC".to_string());
    let mut http_stats = PerformanceStats::new("HTTP RPC".to_string());
    let mut ws_stats = PerformanceStats::new("WebSocket".to_string());
    
    // Setup HTTP provider
    info!("\n🔧 Setting up HTTP RPC connection...");
    let http_provider = match Provider::<Http>::try_from("http://127.0.0.1:8545") {
        Ok(provider) => {
            info!("✅ HTTP RPC connected");
            Some(provider)
        }
        Err(e) => {
            warn!("❌ HTTP RPC failed: {}", e);
            None
        }
    };
    
    // Setup WebSocket provider
    info!("🔧 Setting up WebSocket connection...");
    let ws_provider = match Ws::connect("ws://127.0.0.1:8546").await {
        Ok(ws) => {
            let provider = Provider::new(ws);
            info!("✅ WebSocket connected");
            Some(provider)
        }
        Err(e) => {
            warn!("❌ WebSocket failed: {}", e);
            None
        }
    };
    
    // Setup IPC client
    info!("🔧 Setting up IPC connection...");
    let ipc_client = match FullTxIpcClient::new(Some("/tmp/reth.ipc")) {
        Ok(client) => {
            match client.start_monitoring().await {
                Ok(()) => {
                    info!("✅ IPC connected");
                    Some(client)
                }
                Err(e) => {
                    warn!("❌ IPC connection failed: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            warn!("❌ IPC client creation failed: {}", e);
            None
        }
    };
    
    // Give connections time to stabilize
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    info!("\n📊 Starting comparative performance test...");
    info!("Test duration: 90 seconds");
    
    let test_duration = Duration::from_secs(90);
    let test_start = Instant::now();
    let mut test_hashes = Vec::new();
    
    // Test loop
    while test_start.elapsed() < test_duration {
        let cycle_start = Instant::now();
        
        // Test 1: IPC Method
        if let Some(ref ipc) = ipc_client {
            let ipc_start = Instant::now();
            match ipc.get_full_transactions(10).await {
                Ok(transactions) if !transactions.is_empty() => {
                    let latency = ipc_start.elapsed().as_micros() as u64;
                    ipc_stats.add_measurement(latency, true);
                    
                    // Collect hashes for other methods to test
                    for tx in &transactions[..3.min(transactions.len())] {
                        if let Ok(hash) = tx.hash.parse::<H256>() {
                            test_hashes.push(hash);
                        }
                    }
                }
                Ok(_) => {
                    // No transactions available, still count as measurement
                    let latency = ipc_start.elapsed().as_micros() as u64;
                    ipc_stats.add_measurement(latency, false);
                }
                Err(_) => {
                    let latency = ipc_start.elapsed().as_micros() as u64;
                    ipc_stats.add_measurement(latency, false);
                }
            }
        }
        
        // Test 2: HTTP RPC Method (test with known hashes)
        if let Some(ref http) = http_provider {
            if !test_hashes.is_empty() {
                let hash = test_hashes[test_hashes.len() / 2]; // Use middle hash
                let http_start = Instant::now();
                match http.get_transaction(hash).await {
                    Ok(Some(_)) => {
                        let latency = http_start.elapsed().as_micros() as u64;
                        http_stats.add_measurement(latency, true);
                    }
                    Ok(None) => {
                        let latency = http_start.elapsed().as_micros() as u64;
                        http_stats.add_measurement(latency, false);
                    }
                    Err(_) => {
                        let latency = http_start.elapsed().as_micros() as u64;
                        http_stats.add_measurement(latency, false);
                    }
                }
            }
        }
        
        // Test 3: WebSocket Method (also test with known hashes)
        if let Some(ref ws) = ws_provider {
            if !test_hashes.is_empty() {
                let hash = test_hashes[0]; // Use first hash
                let ws_start = Instant::now();
                match ws.get_transaction(hash).await {
                    Ok(Some(_)) => {
                        let latency = ws_start.elapsed().as_micros() as u64;
                        ws_stats.add_measurement(latency, true);
                    }
                    Ok(None) => {
                        let latency = ws_start.elapsed().as_micros() as u64;
                        ws_stats.add_measurement(latency, false);
                    }
                    Err(_) => {
                        let latency = ws_start.elapsed().as_micros() as u64;
                        ws_stats.add_measurement(latency, false);
                    }
                }
            }
        }
        
        // Limit hash collection to prevent memory growth
        if test_hashes.len() > 100 {
            test_hashes.drain(0..50);
        }
        
        // Progress report every 30 seconds
        if test_start.elapsed().as_secs() % 30 == 0 && cycle_start.elapsed() < Duration::from_secs(1) {
            let progress = test_start.elapsed().as_secs_f64() / test_duration.as_secs_f64() * 100.0;
            info!("  📈 Test progress: {:.1}%", progress);
        }
        
        // Small delay to prevent overwhelming the node
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 COMPARATIVE PERFORMANCE RESULTS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // IPC Results
    let (ipc_avg, ipc_min, ipc_max, ipc_success, ipc_total) = ipc_stats.get_stats();
    info!("\n🔌 IPC Performance:");
    if ipc_success > 0 {
        info!("   • Average latency: {:.1}μs ({:.3}ms)", ipc_avg, ipc_avg / 1000.0);
        info!("   • Min latency: {}μs ({:.3}ms)", ipc_min, ipc_min as f64 / 1000.0);
        info!("   • Max latency: {}μs ({:.3}ms)", ipc_max, ipc_max as f64 / 1000.0);
        info!("   • Success rate: {}/{} ({:.1}%)", ipc_success, ipc_total, 
              ipc_success as f64 / ipc_total as f64 * 100.0);
    } else {
        info!("   • No successful measurements");
    }
    
    // HTTP Results
    let (http_avg, http_min, http_max, http_success, http_total) = http_stats.get_stats();
    info!("\n🌐 HTTP RPC Performance:");
    if http_success > 0 {
        info!("   • Average latency: {:.1}μs ({:.3}ms)", http_avg, http_avg / 1000.0);
        info!("   • Min latency: {}μs ({:.3}ms)", http_min, http_min as f64 / 1000.0);
        info!("   • Max latency: {}μs ({:.3}ms)", http_max, http_max as f64 / 1000.0);
        info!("   • Success rate: {}/{} ({:.1}%)", http_success, http_total,
              http_success as f64 / http_total as f64 * 100.0);
    } else {
        info!("   • No successful measurements");
    }
    
    // WebSocket Results  
    let (ws_avg, ws_min, ws_max, ws_success, ws_total) = ws_stats.get_stats();
    info!("\n📡 WebSocket Performance:");
    if ws_success > 0 {
        info!("   • Average latency: {:.1}μs ({:.3}ms)", ws_avg, ws_avg / 1000.0);
        info!("   • Min latency: {}μs ({:.3}ms)", ws_min, ws_min as f64 / 1000.0);
        info!("   • Max latency: {}μs ({:.3}ms)", ws_max, ws_max as f64 / 1000.0);
        info!("   • Success rate: {}/{} ({:.1}%)", ws_success, ws_total,
              ws_success as f64 / ws_total as f64 * 100.0);
    } else {
        info!("   • No successful measurements");
    }
    
    // Comparison
    info!("\n🏆 Performance Ranking:");
    let mut methods = Vec::new();
    if ipc_success > 0 { methods.push(("IPC", ipc_avg)); }
    if http_success > 0 { methods.push(("HTTP RPC", http_avg)); }
    if ws_success > 0 { methods.push(("WebSocket", ws_avg)); }
    
    methods.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    
    for (i, (method, avg_latency)) in methods.iter().enumerate() {
        let rank = match i {
            0 => "🥇",
            1 => "🥈", 
            2 => "🥉",
            _ => "📊",
        };
        info!("   {} {}: {:.3}ms average", rank, method, avg_latency / 1000.0);
    }
    
    info!("\n💡 Key Findings:");
    if methods.is_empty() {
        warn!("   ⚠️  No successful measurements for any method");
        warn!("   Check that Reth node is running and accessible");
    } else {
        let fastest = &methods[0];
        info!("   • Fastest method: {} ({:.3}ms)", fastest.0, fastest.1 / 1000.0);
        
        if methods.len() > 1 {
            let slowest = &methods[methods.len() - 1];
            let speedup = slowest.1 / fastest.1;
            info!("   • Speed advantage: {:.1}x faster than {}", speedup, slowest.0);
        }
        
        if fastest.1 < 1000.0 {
            info!("   ✅ Sub-millisecond latency achieved with {}", fastest.0);
        } else {
            info!("   ⚠️  All methods above 1ms latency");
        }
    }
    
    info!("\n📋 Test Validity:");
    info!("   • Test duration: {:.1}s", test_duration.as_secs_f64());
    info!("   • Total transaction hashes tested: {}", test_hashes.len());
    info!("   • Results are based on actual timing measurements");
    info!("   • All timing uses std::time::Instant for precision");
    
    Ok(())
}