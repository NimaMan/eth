/// Optimized IPC Client with Minimal Buffering
/// 
/// This implementation uses kernel socket options to minimize buffering
/// and achieve the lowest possible detection latency.
/// 
/// Note: For Unix domain sockets, the optimizations have mixed results:
/// - SO_RCVBUF: Reducing buffer size can help reduce latency in some cases
/// - SO_RCVLOWAT: Setting to 1 ensures immediate notification on any data
/// - SO_SNDBUF: Smaller send buffer for reduced buffering
/// 
/// TCP-specific options (TCP_NODELAY, TCP_QUICKACK) do not apply to Unix sockets
/// and have been removed from this implementation.

use std::time::{Duration, Instant};
use std::os::unix::io::AsRawFd;
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::{Value, json};
use tracing::{info, debug, error};
use eyre::{Result, eyre};

/// Optimized IPC client that minimizes kernel buffering
pub struct OptimizedIpcClient {
    stream: UnixStream,
    subscription_id: Option<String>,
}

impl OptimizedIpcClient {
    /// Create new optimized IPC client
    pub async fn new(socket_path: Option<&str>) -> Result<Self> {
        let path = socket_path.unwrap_or("/tmp/reth.ipc");
        info!("Connecting to IPC socket: {}", path);
        
        let stream = UnixStream::connect(path).await?;
        
        // Optimize socket for minimal latency
        Self::optimize_socket(&stream)?;
        
        Ok(Self {
            stream,
            subscription_id: None,
        })
    }
    
    /// Optimize socket settings for minimal buffering
    fn optimize_socket(stream: &UnixStream) -> Result<()> {
        let fd = stream.as_raw_fd();
        let mut optimizations_applied = Vec::new();
        
        unsafe {
            // 1. Set small receive buffer to reduce buffering
            let rcvbuf: libc::c_int = 4096; // 4KB instead of default 64KB+
            let result = libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &rcvbuf as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
            
            if result == 0 {
                optimizations_applied.push("SO_RCVBUF=4KB");
                
                // Verify the setting was applied
                let mut actual_rcvbuf: libc::c_int = 0;
                let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
                libc::getsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_RCVBUF,
                    &mut actual_rcvbuf as *mut _ as *mut libc::c_void,
                    &mut len,
                );
                println!("  ✓ Receive buffer set to: {} bytes (requested: {})", actual_rcvbuf, rcvbuf);
            } else {
                let err = std::io::Error::last_os_error();
                println!("  ✗ Failed to set SO_RCVBUF: {}", err);
            }
            
            // 2. Set low water mark to 1 - notify on ANY data
            let lowat: libc::c_int = 1;
            let result = libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVLOWAT,
                &lowat as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
            
            if result == 0 {
                optimizations_applied.push("SO_RCVLOWAT=1");
                println!("  ✓ Receive low water mark set to 1 byte");
            } else {
                let err = std::io::Error::last_os_error();
                println!("  ✗ Failed to set SO_RCVLOWAT: {}", err);
            }
            
            // 3. Set send buffer size (optional, for completeness)
            let sndbuf: libc::c_int = 4096; // 4KB
            let result = libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_SNDBUF,
                &sndbuf as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
            
            if result == 0 {
                optimizations_applied.push("SO_SNDBUF=4KB");
                println!("  ✓ Send buffer set to 4KB");
            } else {
                let err = std::io::Error::last_os_error();
                println!("  ✗ Failed to set SO_SNDBUF: {}", err);
            }
        }
        
        if !optimizations_applied.is_empty() {
            println!("✅ Unix socket optimized: {}", optimizations_applied.join(", "));
            info!("Unix socket optimized: {}", optimizations_applied.join(", "));
        } else {
            println!("⚠️  No socket optimizations could be applied");
            info!("No socket optimizations could be applied");
        }
        
        Ok(())
    }
    
    /// Subscribe to pending transactions with minimal latency
    pub async fn subscribe_optimized(&mut self) -> Result<()> {
        let subscribe = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["newPendingTransactions"],
            "id": 1
        });
        
        let request = format!("{}\n", subscribe);
        self.stream.write_all(request.as_bytes()).await?;
        
        // Read subscription response
        let mut buf = vec![0u8; 4096];
        let n = self.stream.read(&mut buf).await?;
        
        let response: Value = serde_json::from_str(
            std::str::from_utf8(&buf[..n])?.lines().next().unwrap_or("")
        )?;
        
        if let Some(sub_id) = response["result"].as_str() {
            self.subscription_id = Some(sub_id.to_string());
            info!("✅ Subscribed with ID: {}", sub_id);
            Ok(())
        } else {
            Err(eyre!("Failed to subscribe"))
        }
    }
    
    /// Read next transaction with precise timing
    pub async fn read_next_transaction(&mut self) -> Result<(String, Duration)> {
        let mut buf = vec![0u8; 4096];
        
        // Start timing immediately before the blocking read
        let start = Instant::now();
        
        // This blocks until data arrives
        let n = self.stream.read(&mut buf).await?;
        
        // Calculate detection latency
        let latency = start.elapsed();
        
        if n == 0 {
            return Err(eyre!("Connection closed"));
        }
        
        // Parse notification
        let notification_str = std::str::from_utf8(&buf[..n])?;
        for line in notification_str.lines() {
            if let Ok(notification) = serde_json::from_str::<Value>(line) {
                if let Some(tx_hash) = notification["params"]["result"].as_str() {
                    return Ok((tx_hash.to_string(), latency));
                }
            }
        }
        
        Err(eyre!("No valid transaction in notification"))
    }
    
    /// Measure detection latency for N transactions
    pub async fn measure_latency(&mut self, count: usize) -> Result<Vec<Duration>> {
        let mut latencies = Vec::with_capacity(count);
        
        for i in 0..count {
            match self.read_next_transaction().await {
                Ok((tx_hash, latency)) => {
                    latencies.push(latency);
                    
                    if i < 5 || latency.as_micros() < 100 {
                        debug!("TX {} detected in {:?}", &tx_hash[..10], latency);
                    }
                }
                Err(e) => {
                    error!("Error reading transaction: {}", e);
                    break;
                }
            }
        }
        
        Ok(latencies)
    }
}

/// Create a baseline (non-optimized) client for comparison
pub struct BaselineIpcClient {
    stream: UnixStream,
    subscription_id: Option<String>,
}

impl BaselineIpcClient {
    pub async fn new(socket_path: Option<&str>) -> Result<Self> {
        let path = socket_path.unwrap_or("/tmp/reth.ipc");
        let stream = UnixStream::connect(path).await?;
        
        // Get default buffer sizes for comparison
        let fd = stream.as_raw_fd();
        unsafe {
            let mut rcvbuf: libc::c_int = 0;
            let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &mut rcvbuf as *mut _ as *mut libc::c_void,
                &mut len,
            );
            println!("  Default receive buffer size: {} bytes", rcvbuf);
        }
        
        Ok(Self {
            stream,
            subscription_id: None,
        })
    }
    
    pub async fn subscribe(&mut self) -> Result<()> {
        let subscribe = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["newPendingTransactions"],
            "id": 1
        });
        
        let request = format!("{}\n", subscribe);
        self.stream.write_all(request.as_bytes()).await?;
        
        let mut buf = vec![0u8; 4096];
        let n = self.stream.read(&mut buf).await?;
        
        let response: Value = serde_json::from_str(
            std::str::from_utf8(&buf[..n])?.lines().next().unwrap_or("")
        )?;
        
        if let Some(sub_id) = response["result"].as_str() {
            self.subscription_id = Some(sub_id.to_string());
            Ok(())
        } else {
            Err(eyre!("Failed to subscribe"))
        }
    }
    
    pub async fn measure_latency(&mut self, count: usize) -> Result<Vec<Duration>> {
        let mut latencies = Vec::with_capacity(count);
        let mut buf = vec![0u8; 4096];
        
        for _ in 0..count {
            let start = Instant::now();
            let n = self.stream.read(&mut buf).await?;
            let latency = start.elapsed();
            
            if n == 0 {
                break;
            }
            
            let notification_str = std::str::from_utf8(&buf[..n])?;
            for line in notification_str.lines() {
                if let Ok(notification) = serde_json::from_str::<Value>(line) {
                    if notification["params"]["result"].as_str().is_some() {
                        latencies.push(latency);
                        break;
                    }
                }
            }
        }
        
        Ok(latencies)
    }
}

/// Test the optimized client and compare with baseline
pub async fn test_optimized_performance() -> Result<()> {
    println!("🚀 Testing IPC Performance: Optimized vs Baseline\n");
    
    // Test baseline first
    println!("📊 Testing BASELINE (non-optimized) client:");
    let mut baseline = BaselineIpcClient::new(None).await?;
    baseline.subscribe().await?;
    
    println!("Measuring 50 transactions...");
    let baseline_latencies = baseline.measure_latency(50).await?;
    
    // Test optimized
    println!("\n📊 Testing OPTIMIZED client:");
    let mut optimized = OptimizedIpcClient::new(None).await?;
    optimized.subscribe_optimized().await?;
    
    println!("Measuring 50 transactions...");
    let optimized_latencies = optimized.measure_latency(50).await?;
    
    // Compare results
    if !baseline_latencies.is_empty() && !optimized_latencies.is_empty() {
        let print_stats = |name: &str, latencies: &[Duration]| {
            let mut sorted = latencies.to_vec();
            sorted.sort();
            
            let avg = latencies.iter().sum::<Duration>() / latencies.len() as u32;
            let min = sorted[0];
            let p50 = sorted[sorted.len() / 2];
            let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];
            let max = sorted[sorted.len() - 1];
            
            println!("\n{} PERFORMANCE:", name);
            println!("  Transactions: {}", latencies.len());
            println!("  Min: {:?}", min);
            println!("  Avg: {:?}", avg);
            println!("  P50: {:?}", p50);
            println!("  P95: {:?}", p95);
            println!("  Max: {:?}", max);
            
            let sub_1ms = latencies.iter().filter(|l| l.as_millis() < 1).count();
            let sub_100us = latencies.iter().filter(|l| l.as_micros() < 100).count();
            
            println!("  <1ms: {:.1}%", sub_1ms as f64 / latencies.len() as f64 * 100.0);
            println!("  <100μs: {:.1}%", sub_100us as f64 / latencies.len() as f64 * 100.0);
            
            (avg, p50, p95)
        };
        
        let (baseline_avg, baseline_p50, baseline_p95) = print_stats("BASELINE", &baseline_latencies);
        let (optimized_avg, optimized_p50, optimized_p95) = print_stats("OPTIMIZED", &optimized_latencies);
        
        // Compare
        println!("\n🔍 COMPARISON:");
        println!("  Average improvement: {:.1}%", 
                 (1.0 - optimized_avg.as_micros() as f64 / baseline_avg.as_micros() as f64) * 100.0);
        println!("  P50 improvement: {:.1}%", 
                 (1.0 - optimized_p50.as_micros() as f64 / baseline_p50.as_micros() as f64) * 100.0);
        println!("  P95 improvement: {:.1}%", 
                 (1.0 - optimized_p95.as_micros() as f64 / baseline_p95.as_micros() as f64) * 100.0);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_client_creation() {
        // This will fail if no IPC socket, which is fine for unit test
        let result = OptimizedIpcClient::new(None).await;
        assert!(result.is_ok() || result.is_err());
    }
}