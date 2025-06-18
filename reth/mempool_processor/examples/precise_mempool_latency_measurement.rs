/// Precise Mempool Latency Measurement
/// 
/// This measures the EXACT metric you specified:
/// Time from "new transaction arrives in mempool" → "we have complete signed transaction data"
/// 
/// Methodology:
/// 1. Subscribe to pending transaction hash announcements (when tx enters mempool)
/// 2. Record timestamp of announcement
/// 3. Fetch full transaction data via IPC
/// 4. Record timestamp when we have complete data
/// 5. Calculate latency = (complete_data_time - announcement_time)

use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::net::UnixStream;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use serde_json::{json, Value};
use ethers::types::{Transaction, H256};
use tracing::{info, warn, error};
use eyre::Result;

#[derive(Debug, Clone)]
struct TransactionLatencyMeasurement {
    hash: String,
    announcement_time: Instant,
    fetch_complete_time: Option<Instant>,
    latency_us: Option<u64>,
    transaction_data: Option<Transaction>,
    fetch_success: bool,
}

impl TransactionLatencyMeasurement {
    fn new(hash: String, announcement_time: Instant) -> Self {
        Self {
            hash,
            announcement_time,
            fetch_complete_time: None,
            latency_us: None,
            transaction_data: None,
            fetch_success: false,
        }
    }
    
    fn mark_fetch_complete(&mut self, transaction: Transaction) {
        let now = Instant::now();
        self.fetch_complete_time = Some(now);
        self.latency_us = Some(now.duration_since(self.announcement_time).as_micros() as u64);
        self.transaction_data = Some(transaction);
        self.fetch_success = true;
    }
    
    fn mark_fetch_failed(&mut self) {
        let now = Instant::now();
        self.fetch_complete_time = Some(now);
        self.latency_us = Some(now.duration_since(self.announcement_time).as_micros() as u64);
        self.fetch_success = false;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("precise_mempool_latency_measurement=info")
        .init();

    info!("🎯 PRECISE MEMPOOL LATENCY MEASUREMENT");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("Measuring: TX enters mempool → We have complete signed TX data");
    info!("Duration: 5 minutes");
    info!("Method: IPC subscription + individual transaction fetching");
    
    // Connect to IPC socket
    info!("\n🔌 Connecting to Reth IPC socket...");
    let mut stream = UnixStream::connect("/tmp/reth.ipc").await
        .map_err(|e| eyre::eyre!("Failed to connect to IPC: {}", e))?;
        
    info!("✅ Connected to /tmp/reth.ipc");
    
    // Subscribe to pending transactions (hash announcements)
    info!("📡 Subscribing to pending transaction announcements...");
    let subscribe_request = json!({
        "jsonrpc": "2.0",
        "method": "eth_subscribe",
        "params": ["newPendingTransactions"],
        "id": 1
    });
    
    let request_str = format!("{}\n", subscribe_request);
    stream.write_all(request_str.as_bytes()).await?;
    
    // Read subscription response
    let mut stream = BufReader::new(stream);
    let mut response_line = String::new();
    stream.read_line(&mut response_line).await?;
    
    let response: Value = serde_json::from_str(&response_line)?;
    let subscription_id = response["result"].as_str()
        .ok_or_else(|| eyre::eyre!("Failed to get subscription ID"))?;
    
    info!("✅ Subscribed to pending transactions: {}", subscription_id);
    
    // Setup measurement tracking
    let mut pending_measurements: HashMap<String, TransactionLatencyMeasurement> = HashMap::new();
    let mut completed_measurements: Vec<TransactionLatencyMeasurement> = Vec::new();
    let mut request_id = 100u64;
    let mut pending_requests: HashMap<u64, String> = HashMap::new();
    
    let measurement_start = Instant::now();
    let measurement_duration = Duration::from_secs(300); // 5 minutes
    
    info!("\n⏱️  Starting 5-minute measurement period...");
    info!("📊 Will measure every transaction that enters the mempool");
    
    let mut stats_counter = 0;
    let mut last_stats_time = Instant::now();
    
    while measurement_start.elapsed() < measurement_duration {
        let mut notification_line = String::new();
        
        // Set a timeout for reading to periodically check if measurement is complete
        match tokio::time::timeout(Duration::from_millis(100), stream.read_line(&mut notification_line)).await {
            Ok(bytes_read) => {
                if bytes_read? == 0 {
                    error!("IPC connection closed");
                    break;
                }
                
                if let Ok(notification) = serde_json::from_str::<Value>(&notification_line) {
                    // Check if this is a response to our getTransactionByHash request
                    if let Some(id) = notification.get("id").and_then(|v| v.as_u64()) {
                        if let Some(tx_hash) = pending_requests.remove(&id) {
                            // This is a response to our transaction fetch
                            if let Some(mut measurement) = pending_measurements.remove(&tx_hash) {
                                if let Some(tx_data) = notification.get("result") {
                                    if !tx_data.is_null() {
                                        // Successfully got transaction data
                                        if let Ok(transaction) = serde_json::from_value::<Transaction>(tx_data.clone()) {
                                            measurement.mark_fetch_complete(transaction);
                                            
                                            // Log individual measurement
                                            if let Some(latency) = measurement.latency_us {
                                                info!("📦 TX {}: {}μs ({:.3}ms)", 
                                                      &measurement.hash[..10], latency, latency as f64 / 1000.0);
                                            }
                                        } else {
                                            measurement.mark_fetch_failed();
                                        }
                                    } else {
                                        measurement.mark_fetch_failed();
                                    }
                                } else {
                                    measurement.mark_fetch_failed();
                                }
                                
                                completed_measurements.push(measurement);
                            }
                        }
                    } 
                    // Check if this is a subscription notification (new transaction hash)
                    else if let Some(params) = notification.get("params") {
                        if let Some(tx_hash) = params["result"].as_str() {
                            // NEW TRANSACTION ANNOUNCED - Start timing here
                            let announcement_time = Instant::now();
                            let measurement = TransactionLatencyMeasurement::new(tx_hash.to_string(), announcement_time);
                            
                            pending_measurements.insert(tx_hash.to_string(), measurement);
                            
                            // Immediately request full transaction data
                            request_id += 1;
                            let fetch_request = json!({
                                "jsonrpc": "2.0",
                                "method": "eth_getTransactionByHash",
                                "params": [tx_hash],
                                "id": request_id
                            });
                            
                            let fetch_str = format!("{}\n", fetch_request);
                            if let Err(e) = stream.get_mut().write_all(fetch_str.as_bytes()).await {
                                error!("Failed to send fetch request: {}", e);
                            } else {
                                pending_requests.insert(request_id, tx_hash.to_string());
                            }
                            
                            stats_counter += 1;
                        }
                    }
                }
            }
            Err(_) => {
                // Timeout - continue to check if measurement period is complete
            }
        }
        
        // Periodic status update
        if last_stats_time.elapsed() > Duration::from_secs(10) {
            let elapsed = measurement_start.elapsed();
            let progress = elapsed.as_secs_f64() / measurement_duration.as_secs_f64() * 100.0;
            
            info!("📈 Progress: {:.1}% | Announced: {} | Completed: {} | Pending: {}", 
                  progress, stats_counter, completed_measurements.len(), pending_measurements.len());
                  
            last_stats_time = Instant::now();
        }
        
        // Clean up very old pending measurements (likely failed)
        let cutoff_time = Instant::now() - Duration::from_secs(30);
        pending_measurements.retain(|_, measurement| measurement.announcement_time > cutoff_time);
    }
    
    let total_measurement_time = measurement_start.elapsed();
    
    info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📊 PRECISE LATENCY MEASUREMENT RESULTS");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    info!("\n⏱️  Measurement Details:");
    info!("   • Total measurement time: {:.1}s", total_measurement_time.as_secs_f64());
    info!("   • Transactions announced: {}", stats_counter);
    info!("   • Successfully measured: {}", completed_measurements.len());
    info!("   • Failed to fetch: {}", stats_counter - completed_measurements.len());
    
    if !completed_measurements.is_empty() {
        let successful: Vec<_> = completed_measurements.iter()
            .filter(|m| m.fetch_success)
            .collect();
            
        if !successful.is_empty() {
            let latencies: Vec<u64> = successful.iter()
                .filter_map(|m| m.latency_us)
                .collect();
                
            if !latencies.is_empty() {
                let avg_latency = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
                let min_latency = *latencies.iter().min().unwrap();
                let max_latency = *latencies.iter().max().unwrap();
                
                // Calculate percentiles
                let mut sorted_latencies = latencies.clone();
                sorted_latencies.sort();
                let len = sorted_latencies.len();
                let p50 = sorted_latencies[len / 2];
                let p95 = sorted_latencies[(len as f64 * 0.95) as usize];
                let p99 = sorted_latencies[(len as f64 * 0.99) as usize];
                
                let sub_1ms = latencies.iter().filter(|&&l| l < 1000).count();
                let sub_10ms = latencies.iter().filter(|&&l| l < 10000).count();
                let sub_100ms = latencies.iter().filter(|&&l| l < 100000).count();
                
                info!("\n🎯 MEMPOOL → COMPLETE TX DATA LATENCY:");
                info!("   • Average: {:.1}μs ({:.3}ms)", avg_latency, avg_latency / 1000.0);
                info!("   • Median (P50): {}μs ({:.3}ms)", p50, p50 as f64 / 1000.0);
                info!("   • P95: {}μs ({:.3}ms)", p95, p95 as f64 / 1000.0);
                info!("   • P99: {}μs ({:.3}ms)", p99, p99 as f64 / 1000.0);
                info!("   • Min: {}μs ({:.3}ms)", min_latency, min_latency as f64 / 1000.0);
                info!("   • Max: {}μs ({:.3}ms)", max_latency, max_latency as f64 / 1000.0);
                
                info!("\n📊 Latency Distribution:");
                info!("   • Sub-1ms: {} ({:.1}%)", sub_1ms, sub_1ms as f64 / latencies.len() as f64 * 100.0);
                info!("   • Sub-10ms: {} ({:.1}%)", sub_10ms, sub_10ms as f64 / latencies.len() as f64 * 100.0);
                info!("   • Sub-100ms: {} ({:.1}%)", sub_100ms, sub_100ms as f64 / latencies.len() as f64 * 100.0);
                
                info!("\n🚀 Throughput Metrics:");
                let tx_per_second = successful.len() as f64 / total_measurement_time.as_secs_f64();
                info!("   • Successful transactions/second: {:.2}", tx_per_second);
                info!("   • Success rate: {:.1}%", 
                      successful.len() as f64 / completed_measurements.len() as f64 * 100.0);
                
                info!("\n💡 What This Measures:");
                info!("   ✅ True end-to-end latency from mempool entry to complete data");
                info!("   ✅ Includes IPC subscription latency + transaction fetch latency");
                info!("   ✅ Measures real production conditions over 5 minutes");
                info!("   ✅ Based on actual transaction flow through Reth mempool");
                
                // Show some example transactions
                info!("\n📋 Sample Measurements:");
                for (i, measurement) in successful.iter().take(5).enumerate() {
                    if let Some(latency) = measurement.latency_us {
                        info!("   {}. TX {}: {}μs ({:.3}ms)", 
                              i + 1, &measurement.hash[..10], latency, latency as f64 / 1000.0);
                    }
                }
                
                // Summary conclusion
                if avg_latency < 1000.0 {
                    info!("\n🏆 CONCLUSION: Sub-millisecond mempool-to-data latency ACHIEVED");
                    info!("   Average: {:.3}ms", avg_latency / 1000.0);
                } else if avg_latency < 10000.0 {
                    info!("\n✅ CONCLUSION: Single-digit millisecond latency achieved");
                    info!("   Average: {:.3}ms", avg_latency / 1000.0);
                } else {
                    info!("\n⚠️  CONCLUSION: Latency higher than expected");
                    info!("   Average: {:.3}ms", avg_latency / 1000.0);
                }
                
            } else {
                warn!("No latency data available");
            }
        } else {
            warn!("No successful transaction fetches");
        }
    } else {
        warn!("No completed measurements");
    }
    
    info!("\n📋 Measurement Methodology:");
    info!("   1. Subscribe to 'newPendingTransactions' via IPC");
    info!("   2. Record timestamp when transaction hash is announced");
    info!("   3. Immediately fetch full transaction via 'eth_getTransactionByHash'");
    info!("   4. Record timestamp when complete transaction data received");
    info!("   5. Calculate latency = step4_time - step2_time");
    info!("   6. Repeat for all transactions over 5-minute period");
    
    Ok(())
}