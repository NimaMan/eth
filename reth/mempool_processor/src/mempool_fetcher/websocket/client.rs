/// WebSocket Client Implementation
/// 
/// Connects to Reth node via WebSocket for real-time transaction streaming.
/// This is the current production method with good balance of performance and reliability.

use std::time::Instant;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use tokio_tungstenite::tungstenite::Message;
use futures_util::{StreamExt, SinkExt};
use serde_json::{Value, json};
use tracing::{info, debug, error, warn};
use eyre::{Result, eyre};
use url::Url;

use crate::mempool_fetcher::types::TransactionView;

/// WebSocket client for real-time transaction streaming
pub struct WebSocketClient {
    /// WebSocket URL (e.g., ws://localhost:8546)
    ws_url: String,
    
    /// HTTP URL for fetching full transaction data
    http_url: String,
    
    /// Transaction sender channel
    tx_sender: mpsc::Sender<WebSocketTransaction>,
    
    /// Transaction receiver
    tx_receiver: Arc<Mutex<mpsc::Receiver<WebSocketTransaction>>>,
    
    /// Performance statistics
    stats: Arc<RwLock<WebSocketStats>>,
    
    /// Active connection state
    is_connected: Arc<RwLock<bool>>,
}

/// Transaction received via WebSocket with timing information
#[derive(Debug, Clone)]
pub struct WebSocketTransaction {
    /// Transaction hash
    pub hash: String,
    
    /// When notification was received
    pub arrival_time: Instant,
    
    /// When we finished processing
    pub detection_time: Instant,
    
    /// Detection latency in milliseconds
    pub latency_ms: f64,
    
    /// Full transaction data (if available)
    pub tx_data: Option<TransactionView>,
}

/// WebSocket performance statistics
#[derive(Debug, Default, Clone)]
pub struct WebSocketStats {
    /// Total transactions detected
    pub total_transactions: u64,
    
    /// Transactions detected in <10ms
    pub sub_10ms_count: u64,
    
    /// Transactions detected in <50ms
    pub sub_50ms_count: u64,
    
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    
    /// Minimum latency observed
    pub min_latency_ms: Option<f64>,
    
    /// Maximum latency observed
    pub max_latency_ms: Option<f64>,
    
    /// All latencies for percentile calculation
    pub latencies_ms: Vec<f64>,
}

impl WebSocketClient {
    /// Create new WebSocket client
    pub fn new(ws_url: &str, http_url: &str) -> Result<Self> {
        let (tx_sender, tx_receiver) = mpsc::channel(10000);
        
        Ok(Self {
            ws_url: ws_url.to_string(),
            http_url: http_url.to_string(),
            tx_sender,
            tx_receiver: Arc::new(Mutex::new(tx_receiver)),
            stats: Arc::new(RwLock::new(WebSocketStats::default())),
            is_connected: Arc::new(RwLock::new(false)),
        })
    }
    
    /// Connect and start monitoring transactions
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("🌐 Connecting to WebSocket: {}", self.ws_url);
        
        let url = Url::parse(&self.ws_url)
            .map_err(|e| eyre!("Invalid WebSocket URL: {}", e))?;
            
        let (ws_stream, _) = connect_async(url).await
            .map_err(|e| eyre!("Failed to connect to WebSocket: {}", e))?;
            
        info!("✅ Connected to WebSocket");
        
        // Mark as connected
        *self.is_connected.write().await = true;
        
        // Start monitoring loop
        let tx_sender = self.tx_sender.clone();
        let stats = self.stats.clone();
        let is_connected = self.is_connected.clone();
        
        tokio::spawn(async move {
            if let Err(e) = Self::monitor_loop(ws_stream, tx_sender, stats).await {
                error!("WebSocket monitor error: {}", e);
            }
            *is_connected.write().await = false;
        });
        
        Ok(())
    }
    
    /// Main monitoring loop
    async fn monitor_loop(
        ws_stream: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
        tx_sender: mpsc::Sender<WebSocketTransaction>,
        stats: Arc<RwLock<WebSocketStats>>,
    ) -> Result<()> {
        let (mut write, mut read) = ws_stream.split();
        
        // Subscribe to pending transactions
        let subscribe_request = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["newPendingTransactions"],
            "id": 1
        });
        
        write.send(Message::Text(subscribe_request.to_string())).await?;
        
        // Read subscription confirmation
        if let Some(Ok(Message::Text(response))) = read.next().await {
            let response: Value = serde_json::from_str(&response)?;
            let subscription_id = response["result"].as_str()
                .ok_or_else(|| eyre!("Failed to get subscription ID"))?;
                
            info!("📡 Subscribed to pending transactions: {}", subscription_id);
        }
        
        let mut count = 0;
        
        // Monitor transaction stream
        while let Some(message) = read.next().await {
            let arrival_time = Instant::now();
            
            match message {
                Ok(Message::Text(text)) => {
                    if let Ok(notification) = serde_json::from_str::<Value>(&text) {
                        if let Some(params) = notification.get("params") {
                            if let Some(tx_hash) = params["result"].as_str() {
                                count += 1;
                                let detection_time = Instant::now();
                                
                                let latency = detection_time.duration_since(arrival_time);
                                let latency_ms = latency.as_secs_f64() * 1000.0;
                                
                                let ws_tx = WebSocketTransaction {
                                    hash: tx_hash.to_string(),
                                    arrival_time,
                                    detection_time,
                                    latency_ms,
                                    tx_data: None,
                                };
                                
                                // Update stats
                                {
                                    let mut stats_guard = stats.write().await;
                                    stats_guard.total_transactions += 1;
                                    stats_guard.latencies_ms.push(latency_ms);
                                    
                                    // Update average
                                    let sum: f64 = stats_guard.latencies_ms.iter().sum();
                                    stats_guard.avg_latency_ms = sum / stats_guard.latencies_ms.len() as f64;
                                    
                                    // Update thresholds
                                    if latency_ms < 10.0 {
                                        stats_guard.sub_10ms_count += 1;
                                    }
                                    if latency_ms < 50.0 {
                                        stats_guard.sub_50ms_count += 1;
                                    }
                                    
                                    // Update min/max
                                    match stats_guard.min_latency_ms {
                                        None => stats_guard.min_latency_ms = Some(latency_ms),
                                        Some(min) if latency_ms < min => stats_guard.min_latency_ms = Some(latency_ms),
                                        _ => {}
                                    }
                                    match stats_guard.max_latency_ms {
                                        None => stats_guard.max_latency_ms = Some(latency_ms),
                                        Some(max) if latency_ms > max => stats_guard.max_latency_ms = Some(latency_ms),
                                        _ => {}
                                    }
                                }
                                
                                // Send transaction
                                if let Err(e) = tx_sender.try_send(ws_tx) {
                                    warn!("Channel full, dropping transaction: {}", e);
                                }
                                
                                // Log progress
                                if count % 1000 == 0 {
                                    let stats_snapshot = stats.read().await;
                                    info!("WebSocket: {} transactions received, avg parsing time: {:.2}ms", 
                                          count, stats_snapshot.avg_latency_ms);
                                }
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {} // Ignore other message types
            }
        }
        
        Ok(())
    }
    
    /// Get new transactions
    pub async fn get_transactions(&self, max_count: usize) -> Result<Vec<WebSocketTransaction>> {
        let mut receiver = self.tx_receiver.lock().await;
        let mut transactions = Vec::with_capacity(max_count);
        
        while transactions.len() < max_count {
            match receiver.try_recv() {
                Ok(tx) => transactions.push(tx),
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(eyre!("WebSocket channel disconnected"));
                }
            }
        }
        
        Ok(transactions)
    }
    
    /// Get performance statistics
    pub async fn get_stats(&self) -> WebSocketStats {
        self.stats.read().await.clone()
    }
    
    /// Calculate percentiles from statistics
    pub async fn get_percentiles(&self) -> Result<(f64, f64, f64)> {
        let stats = self.stats.read().await;
        
        if stats.latencies_ms.is_empty() {
            return Ok((0.0, 0.0, 0.0));
        }
        
        let mut sorted = stats.latencies_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
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
    
    /// Fetch full transaction data for a hash
    pub async fn fetch_transaction(&self, hash: &str) -> Result<TransactionView> {
        // This would use the HTTP endpoint to fetch full transaction data
        // For now, returning a placeholder
        warn!("Transaction fetching not yet implemented");
        Err(eyre!("Transaction fetching not implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_websocket_client_creation() {
        let client = WebSocketClient::new("ws://localhost:8546", "http://localhost:8545").unwrap();
        assert_eq!(client.ws_url, "ws://localhost:8546");
        assert_eq!(client.http_url, "http://localhost:8545");
    }
}