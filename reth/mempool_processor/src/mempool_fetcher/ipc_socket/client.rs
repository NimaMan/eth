/// IPC Socket Client Implementation
/// 
/// Connects to Reth node via Unix domain socket for transaction detection.
/// This is currently the fastest available method without custom Reth builds.

use std::time::Instant;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncBufReadExt, BufReader};
use serde_json::{Value, json};
use tracing::{info, debug, error, warn};
use eyre::{Result, eyre};

use crate::mempool_fetcher::types::TransactionView;

/// IPC client for connecting to local Reth node
pub struct IpcClient {
    /// Path to IPC socket (typically /tmp/reth.ipc)
    socket_path: String,
    
    /// Transaction sender channel
    tx_sender: mpsc::Sender<IpcTransaction>,
    
    /// Transaction receiver
    tx_receiver: Arc<Mutex<mpsc::Receiver<IpcTransaction>>>,
    
    /// Performance statistics
    stats: Arc<RwLock<IpcStats>>,
    
    /// Active connection state
    is_connected: Arc<RwLock<bool>>,
}

/// Transaction received via IPC with timing information
#[derive(Debug, Clone)]
pub struct IpcTransaction {
    /// Transaction hash
    pub hash: String,
    
    /// When we started reading from socket
    pub read_start: Instant,
    
    /// When notification was fully received
    pub detection_time: Instant,
    
    /// Detection latency in microseconds
    pub latency_us: u64,
    
    /// Full transaction data (if available)
    pub tx_data: Option<TransactionView>,
}

/// IPC performance statistics
#[derive(Debug, Default, Clone)]
pub struct IpcStats {
    /// Total transactions detected
    pub total_transactions: u64,
    
    /// Transactions detected in <1ms
    pub sub_1ms_count: u64,
    
    /// Transactions detected in <100μs
    pub sub_100us_count: u64,
    
    /// Average latency in microseconds
    pub avg_latency_us: u64,
    
    /// Minimum latency observed
    pub min_latency_us: Option<u64>,
    
    /// Maximum latency observed
    pub max_latency_us: Option<u64>,
    
    /// All latencies for percentile calculation
    pub latencies: Vec<u64>,
}

impl IpcClient {
    /// Create new IPC client
    pub fn new(socket_path: Option<&str>) -> Result<Self> {
        let socket_path = socket_path.unwrap_or("/tmp/reth.ipc").to_string();
        let (tx_sender, tx_receiver) = mpsc::channel(10000);
        
        Ok(Self {
            socket_path,
            tx_sender,
            tx_receiver: Arc::new(Mutex::new(tx_receiver)),
            stats: Arc::new(RwLock::new(IpcStats::default())),
            is_connected: Arc::new(RwLock::new(false)),
        })
    }
    
    /// Connect and start monitoring transactions
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("🔌 Connecting to IPC socket: {}", self.socket_path);
        
        let stream = UnixStream::connect(&self.socket_path).await
            .map_err(|e| eyre!("Failed to connect to IPC socket: {}", e))?;
            
        info!("✅ Connected to IPC socket");
        
        // Mark as connected
        *self.is_connected.write().await = true;
        
        // Subscribe to pending transactions
        let mut stream = BufReader::new(stream);
        self.subscribe_to_transactions(&mut stream).await?;
        
        // Start monitoring loop
        let tx_sender = self.tx_sender.clone();
        let stats = self.stats.clone();
        let is_connected = self.is_connected.clone();
        
        tokio::spawn(async move {
            if let Err(e) = Self::monitor_loop(stream, tx_sender, stats).await {
                error!("IPC monitor error: {}", e);
            }
            *is_connected.write().await = false;
        });
        
        Ok(())
    }
    
    /// Subscribe to newPendingTransactions
    async fn subscribe_to_transactions(&self, stream: &mut BufReader<UnixStream>) -> Result<()> {
        let subscribe_request = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["newPendingTransactions"],
            "id": 1
        });
        
        let request_str = format!("{}\n", subscribe_request);
        stream.get_mut().write_all(request_str.as_bytes()).await?;
        
        // Read subscription confirmation
        let mut response_line = String::new();
        stream.read_line(&mut response_line).await?;
        
        let response: Value = serde_json::from_str(&response_line)?;
        let subscription_id = response["result"].as_str()
            .ok_or_else(|| eyre!("Failed to get subscription ID"))?;
            
        info!("📡 Subscribed to pending transactions: {}", subscription_id);
        
        Ok(())
    }
    
    /// Main monitoring loop
    async fn monitor_loop(
        mut stream: BufReader<UnixStream>,
        tx_sender: mpsc::Sender<IpcTransaction>,
        stats: Arc<RwLock<IpcStats>>,
    ) -> Result<()> {
        let mut count = 0;
        
        loop {
            let mut notification_line = String::new();
            let read_start = Instant::now();
            
            if stream.read_line(&mut notification_line).await? == 0 {
                break; // Connection closed
            }
            
            let detection_time = Instant::now();
            
            if let Ok(notification) = serde_json::from_str::<Value>(&notification_line) {
                if let Some(params) = notification.get("params") {
                    if let Some(tx_hash) = params["result"].as_str() {
                        count += 1;
                        
                        let latency = detection_time.duration_since(read_start);
                        let latency_us = latency.as_micros() as u64;
                        
                        let ipc_tx = IpcTransaction {
                            hash: tx_hash.to_string(),
                            read_start,
                            detection_time,
                            latency_us,
                            tx_data: None,
                        };
                        
                        // Update stats
                        {
                            let mut stats_guard = stats.write().await;
                            stats_guard.total_transactions += 1;
                            stats_guard.latencies.push(latency_us);
                            
                            // Update average
                            let sum: u64 = stats_guard.latencies.iter().sum();
                            stats_guard.avg_latency_us = sum / stats_guard.latencies.len() as u64;
                            
                            // Update thresholds
                            if latency_us < 1000 {
                                stats_guard.sub_1ms_count += 1;
                            }
                            if latency_us < 100 {
                                stats_guard.sub_100us_count += 1;
                            }
                            
                            // Update min/max
                            match stats_guard.min_latency_us {
                                None => stats_guard.min_latency_us = Some(latency_us),
                                Some(min) if latency_us < min => stats_guard.min_latency_us = Some(latency_us),
                                _ => {}
                            }
                            match stats_guard.max_latency_us {
                                None => stats_guard.max_latency_us = Some(latency_us),
                                Some(max) if latency_us > max => stats_guard.max_latency_us = Some(latency_us),
                                _ => {}
                            }
                        }
                        
                        // Send transaction
                        if let Err(e) = tx_sender.try_send(ipc_tx) {
                            warn!("Channel full, dropping transaction: {}", e);
                        }
                        
                        // Log progress
                        if count % 1000 == 0 {
                            let stats_snapshot = stats.read().await;
                            info!("IPC: {} transactions, avg latency: {}μs", 
                                  count, stats_snapshot.avg_latency_us);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get new transactions
    pub async fn get_transactions(&self, max_count: usize) -> Result<Vec<IpcTransaction>> {
        let mut receiver = self.tx_receiver.lock().await;
        let mut transactions = Vec::with_capacity(max_count);
        
        while transactions.len() < max_count {
            match receiver.try_recv() {
                Ok(tx) => transactions.push(tx),
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(eyre!("IPC channel disconnected"));
                }
            }
        }
        
        Ok(transactions)
    }
    
    /// Get performance statistics
    pub async fn get_stats(&self) -> IpcStats {
        self.stats.read().await.clone()
    }
    
    /// Calculate percentiles from statistics
    pub async fn get_percentiles(&self) -> Result<(u64, u64, u64)> {
        let stats = self.stats.read().await;
        
        if stats.latencies.is_empty() {
            return Ok((0, 0, 0));
        }
        
        let mut sorted = stats.latencies.clone();
        sorted.sort_unstable();
        
        let p50_idx = sorted.len() / 2;
        let p95_idx = (sorted.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;
        
        let p50 = sorted[p50_idx];
        let p95 = sorted[p95_idx.min(sorted.len() - 1)];
        let p99 = sorted[p99_idx.min(sorted.len() - 1)];
        
        Ok((p50, p95, p99))
    }
    
    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ipc_client_creation() {
        let client = IpcClient::new(None).unwrap();
        assert_eq!(client.socket_path, "/tmp/reth.ipc");
        
        let client = IpcClient::new(Some("/custom/path.ipc")).unwrap();
        assert_eq!(client.socket_path, "/custom/path.ipc");
    }
}