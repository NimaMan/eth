//! IPC-IPC Transaction Measurement Client
//! 
//! Provides reusable components for measuring transaction latency using
//! Unix IPC socket communication to Reth node.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::Write;
use tokio::net::UnixStream;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use serde_json::{json, Value};
use ethers::types::Transaction;
use tracing::{info, warn, error};
use eyre::Result;
use chrono::Utc;

/// Configuration for IPC-IPC measurement
#[derive(Debug, Clone)]
pub struct MeasurementConfig {
    pub ipc_socket_path: String,
    pub target_transaction_count: usize,
    pub log_directory: String,
    pub progress_interval_transactions: usize,
    pub high_latency_threshold_ms: f64,
}

/// Individual transaction latency measurement
#[derive(Debug, Clone)]
pub struct TransactionLatencyMeasurement {
    pub hash: String,
    pub mempool_arrival_time: Instant,
    pub mempool_arrival_timestamp: u64,
    pub fetch_complete_time: Option<Instant>,
    pub fetch_complete_timestamp: Option<u64>,
    pub latency_us: Option<u64>,
    pub fetch_success: bool,
    pub transaction_size_bytes: Option<usize>,
}

/// Aggregated measurement results
#[derive(Debug, Clone)]
pub struct MeasurementResults {
    pub total_transactions: usize,
    pub successful_transactions: usize,
    pub measurement_duration: Duration,
    pub avg_latency_us: f64,
    pub median_latency_us: u64,
    pub min_latency_us: u64,
    pub max_latency_us: u64,
    pub sub_1ms_count: usize,
    pub sub_1ms_percentage: f64,
    pub transactions_per_second: f64,
    pub success_rate_percentage: f64,
}

impl TransactionLatencyMeasurement {
    /// Create new measurement, starting the timer
    pub fn new(hash: String) -> Self {
        let now = Instant::now();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
            
        Self {
            hash,
            mempool_arrival_time: now,
            mempool_arrival_timestamp: timestamp,
            fetch_complete_time: None,
            fetch_complete_timestamp: None,
            latency_us: None,
            fetch_success: false,
            transaction_size_bytes: None,
        }
    }
    
    /// Mark transaction fetch as complete, stopping the timer
    pub fn mark_fetch_complete(&mut self, transaction: Transaction) {
        let now = Instant::now();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
            
        self.fetch_complete_time = Some(now);
        self.fetch_complete_timestamp = Some(timestamp);
        self.latency_us = Some(now.duration_since(self.mempool_arrival_time).as_micros() as u64);
        self.fetch_success = true;
        
        // Calculate transaction size for analysis
        if let Ok(serialized) = serde_json::to_string(&transaction) {
            self.transaction_size_bytes = Some(serialized.len());
        }
    }
    
    /// Mark transaction fetch as failed
    pub fn mark_fetch_failed(&mut self) {
        let now = Instant::now();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
            
        self.fetch_complete_time = Some(now);
        self.fetch_complete_timestamp = Some(timestamp);
        self.latency_us = Some(now.duration_since(self.mempool_arrival_time).as_micros() as u64);
        self.fetch_success = false;
    }
}

/// IPC-IPC measurement client
pub struct IpcIpcMeasurementClient {
    config: MeasurementConfig,
    stream: Option<BufReader<UnixStream>>,
    csv_file: Option<File>,
    detail_file: Option<File>,
}

impl IpcIpcMeasurementClient {
    /// Create new measurement client
    pub fn new(config: MeasurementConfig) -> Self {
        Self {
            config,
            stream: None,
            csv_file: None,
            detail_file: None,
        }
    }
    
    /// Initialize connection and logging
    pub async fn initialize(&mut self) -> Result<()> {
        // Create log directory
        create_dir_all(&self.config.log_directory)?;
        
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let log_file_path = format!("{}/tx_measurements_{}.csv", self.config.log_directory, timestamp);
        let detailed_log_path = format!("{}/detailed_log_{}.txt", self.config.log_directory, timestamp);
        
        let mut csv_file = File::create(&log_file_path)?;
        let mut detail_file = File::create(&detailed_log_path)?;
        
        // Write CSV headers
        writeln!(csv_file, "tx_hash,mempool_arrival_ms,fetch_complete_ms,latency_us,success,size_bytes")?;
        
        // Write detailed log header
        writeln!(detail_file, "=== IPC-IPC TRANSACTION MEASUREMENT LOG ===")?;
        writeln!(detail_file, "Start time: {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))?;
        writeln!(detail_file, "Method: IPC eth_subscribe → IPC eth_getTransactionByHash")?;
        writeln!(detail_file, "Socket: {}", self.config.ipc_socket_path)?;
        writeln!(detail_file, "")?;
        
        info!("📁 Created log files:");
        info!("   • CSV data: {}", log_file_path);
        info!("   • Detailed log: {}", detailed_log_path);
        
        // Connect to IPC socket
        info!("🔌 Connecting to IPC socket: {}", self.config.ipc_socket_path);
        let mut stream = UnixStream::connect(&self.config.ipc_socket_path).await
            .map_err(|e| eyre::eyre!("Failed to connect to IPC: {}", e))?;
            
        info!("✅ Connected to {}", self.config.ipc_socket_path);
        
        // Subscribe to pending transactions
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
        
        self.stream = Some(stream);
        self.csv_file = Some(csv_file);
        self.detail_file = Some(detail_file);
        
        Ok(())
    }
    
    /// Run measurement for configured number of transactions
    pub async fn run_measurement(&mut self) -> Result<MeasurementResults> {
        let mut pending_measurements: HashMap<String, TransactionLatencyMeasurement> = HashMap::new();
        let mut completed_measurements: Vec<TransactionLatencyMeasurement> = Vec::new();
        let mut request_id = 100u64;
        let mut pending_requests: HashMap<u64, String> = HashMap::new();
        
        let measurement_start = Instant::now();
        let target_count = self.config.target_transaction_count;
        
        info!("⏱️  Starting IPC-IPC measurement of {} transactions...", target_count);
        writeln!(self.detail_file.as_mut().unwrap(), "STARTING MEASUREMENT - Target: {} transactions", target_count)?;
        self.detail_file.as_mut().unwrap().flush()?;
        
        let mut last_progress_log = Instant::now();
        let stream = self.stream.as_mut().unwrap();
        
        while completed_measurements.len() < target_count {
            let mut notification_line = String::new();
            
            match tokio::time::timeout(Duration::from_millis(100), stream.read_line(&mut notification_line)).await {
                Ok(bytes_read) => {
                    if bytes_read? == 0 {
                        error!("IPC connection closed");
                        break;
                    }
                    
                    if let Ok(notification) = serde_json::from_str::<Value>(&notification_line) {
                        // Handle transaction fetch responses
                        if let Some(id) = notification.get("id").and_then(|v| v.as_u64()) {
                            if let Some(tx_hash) = pending_requests.remove(&id) {
                                if let Some(mut measurement) = pending_measurements.remove(&tx_hash) {
                                    let tx_number = completed_measurements.len() + 1;
                                    if let Some(tx_data) = notification.get("result") {
                                        if !tx_data.is_null() {
                                            if let Ok(transaction) = serde_json::from_value::<Transaction>(tx_data.clone()) {
                                                measurement.mark_fetch_complete(transaction);
                                                Self::log_successful_measurement_static(&mut self.detail_file, &mut self.csv_file, &measurement, tx_number)?;
                                            } else {
                                                measurement.mark_fetch_failed();
                                                Self::log_failed_measurement_static(&mut self.detail_file, &measurement, tx_number, "PARSE_FAILED")?;
                                            }
                                        } else {
                                            measurement.mark_fetch_failed();
                                            Self::log_failed_measurement_static(&mut self.detail_file, &measurement, tx_number, "NULL_RESPONSE")?;
                                        }
                                    } else {
                                        measurement.mark_fetch_failed();
                                        Self::log_failed_measurement_static(&mut self.detail_file, &measurement, tx_number, "NO_RESULT")?;
                                    }
                                    
                                    completed_measurements.push(measurement);
                                }
                            }
                        }
                        // Handle new transaction announcements
                        else if let Some(params) = notification.get("params") {
                            if let Some(tx_hash) = params["result"].as_str() {
                                // MEMPOOL ARRIVAL - Start timing
                                let measurement = TransactionLatencyMeasurement::new(tx_hash.to_string());
                                
                                writeln!(self.detail_file.as_mut().unwrap(), 
                                    "MEMPOOL ARRIVAL: {} at {}ms", 
                                    &tx_hash[..10], 
                                    measurement.mempool_arrival_timestamp
                                )?;
                                
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
                            }
                        }
                    }
                }
                Err(_) => {
                    // Timeout - continue
                }
            }
            
            // Progress logging
            if last_progress_log.elapsed() > Duration::from_secs(10) {
                let progress = completed_measurements.len() as f64 / target_count as f64 * 100.0;
                info!("📈 Progress: {:.1}% ({}/{}) | Pending: {} | Elapsed: {:.1}s", 
                    progress, 
                    completed_measurements.len(), 
                    target_count,
                    pending_measurements.len(),
                    measurement_start.elapsed().as_secs_f64()
                );
                
                self.detail_file.as_mut().unwrap().flush()?;
                self.csv_file.as_mut().unwrap().flush()?;
                last_progress_log = Instant::now();
            }
            
            // Clean up old pending measurements (timeout after 30 seconds)
            let cutoff_time = Instant::now() - Duration::from_secs(30);
            pending_measurements.retain(|_, measurement| measurement.mempool_arrival_time > cutoff_time);
        }
        
        let total_measurement_time = measurement_start.elapsed();
        let results = self.calculate_results(&completed_measurements, total_measurement_time)?;
        
        self.detail_file.as_mut().unwrap().flush()?;
        self.csv_file.as_mut().unwrap().flush()?;
        
        Ok(results)
    }
    
    fn log_successful_measurement_static(
        detail_file: &mut Option<File>, 
        csv_file: &mut Option<File>,
        measurement: &TransactionLatencyMeasurement, 
        tx_number: usize
    ) -> Result<()> {
        // Log to detailed file
        writeln!(detail_file.as_mut().unwrap(), 
            "TX #{}: {} | Arrival: {} | Complete: {} | Latency: {}μs | Success: ✅",
            tx_number,
            &measurement.hash[..10],
            measurement.mempool_arrival_timestamp,
            measurement.fetch_complete_timestamp.unwrap_or(0),
            measurement.latency_us.unwrap_or(0)
        )?;
        
        // Log to CSV
        writeln!(csv_file.as_mut().unwrap(),
            "{},{},{},{},{},{}",
            measurement.hash,
            measurement.mempool_arrival_timestamp,
            measurement.fetch_complete_timestamp.unwrap_or(0),
            measurement.latency_us.unwrap_or(0),
            measurement.fetch_success,
            measurement.transaction_size_bytes.unwrap_or(0)
        )?;
        
        info!("📦 TX #{}: {} → {}μs ({:.3}ms)", 
            tx_number,
            &measurement.hash[..10], 
            measurement.latency_us.unwrap_or(0),
            measurement.latency_us.unwrap_or(0) as f64 / 1000.0
        );
        
        Ok(())
    }
    
    fn log_failed_measurement_static(
        detail_file: &mut Option<File>,
        measurement: &TransactionLatencyMeasurement, 
        tx_number: usize, 
        reason: &str
    ) -> Result<()> {
        writeln!(detail_file.as_mut().unwrap(), 
            "TX #{}: {} | {} | Latency: {}μs",
            tx_number,
            &measurement.hash[..10],
            reason,
            measurement.latency_us.unwrap_or(0)
        )?;
        
        Ok(())
    }
    
    fn calculate_results(&mut self, measurements: &[TransactionLatencyMeasurement], duration: Duration) -> Result<MeasurementResults> {
        let successful: Vec<_> = measurements.iter()
            .filter(|m| m.fetch_success)
            .collect();
            
        if successful.is_empty() {
            return Err(eyre::eyre!("No successful measurements"));
        }
        
        let latencies: Vec<u64> = successful.iter()
            .filter_map(|m| m.latency_us)
            .collect();
            
        let avg_latency_us = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
        let min_latency_us = *latencies.iter().min().unwrap();
        let max_latency_us = *latencies.iter().max().unwrap();
        
        let mut sorted = latencies.clone();
        sorted.sort();
        let median_latency_us = sorted[sorted.len() / 2];
        
        let sub_1ms_count = latencies.iter().filter(|&&l| l < 1000).count();
        let sub_1ms_percentage = sub_1ms_count as f64 / latencies.len() as f64 * 100.0;
        
        let transactions_per_second = successful.len() as f64 / duration.as_secs_f64();
        let success_rate_percentage = successful.len() as f64 / measurements.len() as f64 * 100.0;
        
        // Log final statistics
        writeln!(self.detail_file.as_mut().unwrap(), "")?;
        writeln!(self.detail_file.as_mut().unwrap(), "=== FINAL STATISTICS ===")?;
        writeln!(self.detail_file.as_mut().unwrap(), "End time: {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))?;
        writeln!(self.detail_file.as_mut().unwrap(), "Total measurement time: {:.2}s", duration.as_secs_f64())?;
        writeln!(self.detail_file.as_mut().unwrap(), "Transactions measured: {}", measurements.len())?;
        writeln!(self.detail_file.as_mut().unwrap(), "Average latency: {:.1}μs ({:.3}ms)", avg_latency_us, avg_latency_us / 1000.0)?;
        writeln!(self.detail_file.as_mut().unwrap(), "Median latency: {}μs ({:.3}ms)", median_latency_us, median_latency_us as f64 / 1000.0)?;
        writeln!(self.detail_file.as_mut().unwrap(), "Min latency: {}μs", min_latency_us)?;
        writeln!(self.detail_file.as_mut().unwrap(), "Max latency: {}μs", max_latency_us)?;
        writeln!(self.detail_file.as_mut().unwrap(), "Sub-1ms count: {} ({:.1}%)", sub_1ms_count, sub_1ms_percentage)?;
        writeln!(self.detail_file.as_mut().unwrap(), "Success rate: {:.1}%", success_rate_percentage)?;
        writeln!(self.detail_file.as_mut().unwrap(), "Transactions per second: {:.2}", transactions_per_second)?;
        
        Ok(MeasurementResults {
            total_transactions: measurements.len(),
            successful_transactions: successful.len(),
            measurement_duration: duration,
            avg_latency_us,
            median_latency_us,
            min_latency_us,
            max_latency_us,
            sub_1ms_count,
            sub_1ms_percentage,
            transactions_per_second,
            success_rate_percentage,
        })
    }
}