/// IPC Client with Full Transaction Data Support
/// 
/// This client subscribes to pending transactions with full data included,
/// eliminating the need for separate HTTP RPC calls to fetch transaction details.
/// 
/// Two subscription modes:
/// 1. Standard: "newPendingTransactions" - returns only transaction hashes
/// 2. Full: "newPendingTransactions" with includeTransactions=true - returns full tx data

use std::time::Instant;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncBufReadExt, BufReader};
use serde_json::{Value, json};
use tracing::{info, debug, error, warn};
use eyre::{Result, eyre};
use ethers::types::{Transaction, H256, U256};

use crate::mempool_fetcher::types::TransactionView;

/// Full transaction received via IPC
#[derive(Debug, Clone)]
pub struct FullIpcTransaction {
    /// Transaction hash
    pub hash: String,
    
    /// Full ethers Transaction object
    pub transaction: Transaction,
    
    /// Converted TransactionView for compatibility
    pub tx_view: TransactionView,
    
    /// When transaction was detected
    pub detection_time: Instant,
    
    /// Detection latency in microseconds
    pub latency_us: u64,
}

/// IPC client that fetches full transaction data
pub struct FullTxIpcClient {
    /// Path to IPC socket
    socket_path: String,
    
    /// Transaction sender channel
    tx_sender: mpsc::Sender<FullIpcTransaction>,
    
    /// Transaction receiver
    tx_receiver: Arc<Mutex<mpsc::Receiver<FullIpcTransaction>>>,
    
    /// Performance statistics
    stats: Arc<RwLock<IpcStats>>,
    
    /// Active connection state
    is_connected: Arc<RwLock<bool>>,
}

/// Performance statistics
#[derive(Debug, Default, Clone)]
pub struct IpcStats {
    pub total_transactions: u64,
    pub sub_1ms_count: u64,
    pub sub_10ms_count: u64,
    pub avg_latency_us: u64,
    pub min_latency_us: Option<u64>,
    pub max_latency_us: Option<u64>,
    pub latencies: Vec<u64>,
}

impl FullTxIpcClient {
    /// Create new IPC client with full transaction support
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
    
    /// Connect and start monitoring with full transaction data
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("🔌 Connecting to IPC socket for full transactions: {}", self.socket_path);
        
        let stream = UnixStream::connect(&self.socket_path).await
            .map_err(|e| eyre!("Failed to connect to IPC socket: {}", e))?;
            
        info!("✅ Connected to IPC socket");
        
        // Mark as connected
        *self.is_connected.write().await = true;
        
        // Subscribe to pending transactions WITH FULL DATA
        let mut stream = BufReader::new(stream);
        self.subscribe_to_full_transactions(&mut stream).await?;
        
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
    
    /// Subscribe to newPendingTransactions with full transaction data
    async fn subscribe_to_full_transactions(&self, stream: &mut BufReader<UnixStream>) -> Result<()> {
        // Try subscription with full transaction data
        let subscribe_request = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["newPendingTransactions", {"includeTransactions": true}],
            "id": 1
        });
        
        let request_str = format!("{}\n", subscribe_request);
        stream.get_mut().write_all(request_str.as_bytes()).await?;
        
        // Read subscription confirmation
        let mut response_line = String::new();
        stream.read_line(&mut response_line).await?;
        
        let response: Value = serde_json::from_str(&response_line)?;
        
        if let Some(error) = response.get("error") {
            warn!("Full transaction subscription not supported: {}", error);
            
            // Fallback to standard subscription
            return self.subscribe_to_standard_transactions(stream).await;
        }
        
        let subscription_id = response["result"].as_str()
            .ok_or_else(|| eyre!("Failed to get subscription ID"))?;
            
        info!("📡 Subscribed to pending transactions WITH FULL DATA: {}", subscription_id);
        
        Ok(())
    }
    
    /// Fallback: Subscribe to standard pending transactions (hashes only)
    async fn subscribe_to_standard_transactions(&self, stream: &mut BufReader<UnixStream>) -> Result<()> {
        let subscribe_request = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["newPendingTransactions"],
            "id": 2
        });
        
        let request_str = format!("{}\n", subscribe_request);
        stream.get_mut().write_all(request_str.as_bytes()).await?;
        
        let mut response_line = String::new();
        stream.read_line(&mut response_line).await?;
        
        let response: Value = serde_json::from_str(&response_line)?;
        let subscription_id = response["result"].as_str()
            .ok_or_else(|| eyre!("Failed to get subscription ID"))?;
            
        warn!("📡 Fallback: Subscribed to pending transactions (hashes only): {}", subscription_id);
        info!("Will need to fetch full transaction data separately");
        
        Ok(())
    }
    
    /// Main monitoring loop
    async fn monitor_loop(
        mut stream: BufReader<UnixStream>,
        tx_sender: mpsc::Sender<FullIpcTransaction>,
        stats: Arc<RwLock<IpcStats>>,
    ) -> Result<()> {
        let mut count = 0;
        let mut request_id = 100; // Start with ID 100 for eth_getTransactionByHash requests
        let mut pending_requests = std::collections::HashMap::new();
        
        loop {
            let mut notification_line = String::new();
            let read_start = Instant::now();
            
            if stream.read_line(&mut notification_line).await? == 0 {
                break; // Connection closed
            }
            
            let detection_time = Instant::now();
            
            if let Ok(notification) = serde_json::from_str::<Value>(&notification_line) {
                // Check if this is a response to our getTransactionByHash request
                if let Some(id) = notification.get("id").and_then(|v| v.as_u64()) {
                    if let Some((tx_hash, start_time)) = pending_requests.remove(&id) {
                        // This is a response to our transaction fetch request
                        if let Some(tx_data) = notification.get("result") {
                            if !tx_data.is_null() {
                                // Parse the transaction
                                if let Ok(transaction) = serde_json::from_value::<Transaction>(tx_data.clone()) {
                                    let latency = detection_time.duration_since(start_time);
                                    let latency_us = latency.as_micros() as u64;
                                    
                                    // Convert to TransactionView
                                    let tx_view = TransactionView {
                                        hash: transaction.hash.as_bytes().to_vec(),
                                        from: transaction.from.as_bytes().to_vec(),
                                        to: transaction.to.map(|addr| addr.as_bytes().to_vec()),
                                        value: transaction.value,
                                        gas_price: transaction.gas_price,
                                        gas_limit: Some(transaction.gas),
                                        nonce: Some(transaction.nonce),
                                        input_data: Some(transaction.input.to_vec()),
                                    };
                                    
                                    let full_tx = FullIpcTransaction {
                                        hash: tx_hash,
                                        transaction,
                                        tx_view,
                                        detection_time,
                                        latency_us,
                                    };
                                    
                                    // Update stats
                                    Self::update_stats(&stats, latency_us).await;
                                    
                                    // Send transaction
                                    if let Err(e) = tx_sender.try_send(full_tx) {
                                        warn!("Channel full, dropping transaction: {}", e);
                                    }
                                    
                                    // Log progress
                                    if count % 100 == 0 {
                                        let stats_snapshot = stats.read().await;
                                        info!("IPC Full: {} transactions, avg latency: {}μs", 
                                              count, stats_snapshot.avg_latency_us);
                                    }
                                }
                            }
                        }
                    }
                    continue;
                }
                
                // Check if this is a subscription notification
                if let Some(params) = notification.get("params") {
                    // Check if we got full transaction data
                    if let Some(tx_data) = params["result"].as_object() {
                        // We received full transaction data!
                        if let Ok(transaction) = serde_json::from_value::<Transaction>(params["result"].clone()) {
                            count += 1;
                            
                            let latency = detection_time.duration_since(read_start);
                            let latency_us = latency.as_micros() as u64;
                            
                            // Convert to TransactionView
                            let tx_view = TransactionView {
                                hash: transaction.hash.as_bytes().to_vec(),
                                from: transaction.from.as_bytes().to_vec(),
                                to: transaction.to.map(|addr| addr.as_bytes().to_vec()),
                                value: transaction.value,
                                gas_price: transaction.gas_price,
                                gas_limit: Some(transaction.gas),
                                nonce: Some(transaction.nonce),
                                input_data: Some(transaction.input.to_vec()),
                            };
                            
                            let full_tx = FullIpcTransaction {
                                hash: format!("{:?}", transaction.hash),
                                transaction,
                                tx_view,
                                detection_time,
                                latency_us,
                            };
                            
                            // Update stats
                            Self::update_stats(&stats, latency_us).await;
                            
                            // Send transaction
                            if let Err(e) = tx_sender.try_send(full_tx) {
                                warn!("Channel full, dropping transaction: {}", e);
                            }
                        }
                    } else if let Some(tx_hash) = params["result"].as_str() {
                        // We only got a hash, need to fetch full transaction via IPC
                        count += 1;
                        
                        // Send eth_getTransactionByHash request through the same IPC connection
                        request_id += 1;
                        let get_tx_request = json!({
                            "jsonrpc": "2.0",
                            "method": "eth_getTransactionByHash",
                            "params": [tx_hash],
                            "id": request_id
                        });
                        
                        let request_str = format!("{}\n", get_tx_request);
                        stream.get_mut().write_all(request_str.as_bytes()).await?;
                        
                        // Track this request
                        pending_requests.insert(request_id, (tx_hash.to_string(), detection_time));
                        
                        debug!("Fetching full transaction data for: {}", &tx_hash[..10]);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Update statistics
    async fn update_stats(stats: &Arc<RwLock<IpcStats>>, latency_us: u64) {
        let mut stats_guard = stats.write().await;
        stats_guard.total_transactions += 1;
        stats_guard.latencies.push(latency_us);
        
        // Keep only last 1000 for memory efficiency
        if stats_guard.latencies.len() > 1000 {
            stats_guard.latencies.remove(0);
        }
        
        // Update average
        let sum: u64 = stats_guard.latencies.iter().sum();
        stats_guard.avg_latency_us = sum / stats_guard.latencies.len() as u64;
        
        // Update thresholds
        if latency_us < 1000 {
            stats_guard.sub_1ms_count += 1;
        }
        if latency_us < 10000 {
            stats_guard.sub_10ms_count += 1;
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
    
    /// Get new transactions with full data
    pub async fn get_full_transactions(&self, max_count: usize) -> Result<Vec<FullIpcTransaction>> {
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
    
    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_full_tx_client_creation() {
        let client = FullTxIpcClient::new(None).unwrap();
        assert_eq!(client.socket_path, "/tmp/reth.ipc");
    }
}