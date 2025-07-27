use std::time::{Duration, Instant};
use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::net::UnixStream;
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use tokio::time::timeout;
use serde_json::{Value, json};
use tracing::{info, debug, error, warn};
use eyre::{Result, eyre};

/// Transaction with complete data received directly from IPC
#[derive(Debug, Clone)]
pub struct FullTransaction {
    /// Transaction hash
    pub hash: String,
    /// Complete transaction data
    pub tx_data: Value,
    /// When we detected it
    pub detection_time: Instant,
    /// Detection latency in nanoseconds
    pub latency_ns: u64,
}

/// IPC client that receives full transaction data without RPC fallback
pub struct FullTransactionIpcClient {
    /// Path to IPC socket
    socket_path: String,
    /// Transaction sender channel
    tx_sender: mpsc::Sender<FullTransaction>,
    /// Transaction receiver
    tx_receiver: Arc<Mutex<mpsc::Receiver<FullTransaction>>>,
    /// Performance statistics
    stats: Arc<RwLock<IpcClientStats>>,
    /// Queue size counter
    queue_size: Arc<std::sync::atomic::AtomicUsize>,
    /// Connection state
    is_connected: Arc<RwLock<bool>>,
}

/// Performance statistics
#[derive(Debug, Default, Clone)]
pub struct IpcClientStats {
    pub total_transactions: u64,
    pub sub_1ms_count: u64,
    pub sub_100us_count: u64,
    pub sub_10us_count: u64,
    pub avg_latency_ns: u64,
    pub min_latency_ns: Option<u64>,
    pub max_latency_ns: Option<u64>,
    pub latencies: VecDeque<u64>,
    pub sum_latencies: u64,
}

impl FullTransactionIpcClient {
    /// Create new IPC client for full transaction data
    pub fn new(socket_path: Option<&str>) -> Result<Self> {
        let socket_path = socket_path.unwrap_or("/tmp/reth.ipc").to_string();
        let (tx_sender, tx_receiver) = mpsc::channel(50000);
        info!("📦 Created FullTransactionIpcClient - direct transaction data");
        
        Ok(Self {
            socket_path,
            tx_sender,
            tx_receiver: Arc::new(Mutex::new(tx_receiver)),
            stats: Arc::new(RwLock::new(IpcClientStats::default())),
            queue_size: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            is_connected: Arc::new(RwLock::new(false)),
        })
    }
    
    /// Start monitoring with full transaction data
    pub async fn start_monitoring(&self) -> Result<()> {
        info!("🔌 Connecting to Reth IPC for full transactions: {}", self.socket_path);
        
        // Add timeout to connection attempt (5 seconds)
        let stream = match timeout(
            Duration::from_secs(5),
            UnixStream::connect(&self.socket_path)
        ).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => return Err(eyre!("Failed to connect to IPC socket: {}", e)),
            Err(_) => return Err(eyre!("IPC connection timed out after 5 seconds")),
        };
            
        info!("✅ Connected to Reth IPC");
        
        // Mark as connected
        *self.is_connected.write().await = true;
        
        // Subscribe with correct parameter format for full transactions
        let mut stream = BufReader::new(stream);
        self.subscribe_to_full_transactions(&mut stream).await?;
        
        // Start monitoring loop
        let tx_sender = self.tx_sender.clone();
        let stats = self.stats.clone();
        let queue_size = self.queue_size.clone();
        let is_connected = self.is_connected.clone();
        
        tokio::spawn(async move {
            if let Err(e) = Self::monitor_loop(stream, tx_sender, stats, queue_size).await {
                error!("IPC monitor error: {}", e);
            }
            *is_connected.write().await = false;
        });
        
        Ok(())
    }
    
    /// Subscribe to full transactions with proper parameters
    async fn subscribe_to_full_transactions(&self, stream: &mut BufReader<UnixStream>) -> Result<()> {
        // Key insight: Use `true` not `{"includeTransactions": true}`
        let subscribe_request = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["newPendingTransactions", true],
            "id": 1
        });
        
        let request_str = format!("{}\n", subscribe_request);
        stream.get_mut().write_all(request_str.as_bytes()).await?;
        
        // Read subscription confirmation with timeout
        let mut response_line = String::new();
        match timeout(
            Duration::from_secs(5),
            stream.read_line(&mut response_line)
        ).await {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => return Err(eyre!("Failed to read subscription response: {}", e)),
            Err(_) => return Err(eyre!("Subscription response timed out after 5 seconds")),
        };
        
        let response: Value = serde_json::from_str(&response_line)?;
        
        if let Some(error) = response.get("error") {
            error!("❌ Full transaction subscription failed: {}", error);
            return Err(eyre!("Subscription failed: {}", error));
        }
        
        let subscription_id = response["result"].as_str()
            .ok_or_else(|| eyre!("Failed to get subscription ID"))?;
            
        info!("📡 Subscribed to full transactions: {}", subscription_id);
        info!("🚀 FullTransactionIpcClient ready - no RPC fallback needed");
        
        Ok(())
    }
    
    /// Main monitoring loop for full transactions
    async fn monitor_loop(
        mut stream: BufReader<UnixStream>,
        tx_sender: mpsc::Sender<FullTransaction>,
        stats: Arc<RwLock<IpcClientStats>>,
        queue_size: Arc<std::sync::atomic::AtomicUsize>,
    ) -> Result<()> {
        let mut count = 0;
        
        loop {
            let mut notification_line = String::new();
            let read_start = Instant::now();
            
            // Add timeout to read operations (60 seconds for long-running streams)
            match timeout(
                Duration::from_secs(60),
                stream.read_line(&mut notification_line)
            ).await {
                Ok(Ok(0)) => break, // Connection closed
                Ok(Ok(_)) => {}, // Successfully read line
                Ok(Err(e)) => {
                    error!("Failed to read from IPC stream: {}", e);
                    return Err(eyre!("IPC read error: {}", e));
                },
                Err(_) => {
                    warn!("IPC read timed out after 60 seconds, continuing...");
                    continue;
                }
            }
            
            let detection_time = Instant::now();
            let latency_ns = detection_time.duration_since(read_start).as_nanos() as u64;
            
            if let Ok(notification) = serde_json::from_str::<Value>(&notification_line) {
                if let Some(params) = notification.get("params") {
                    if let Some(result) = params.get("result") {
                        count += 1;
                        
                        // This should be a full transaction object, never a hash
                        if result.is_string() {
                            error!("❌ ERROR: Received hash instead of full transaction!");
                            error!("   Check subscription parameters - something is wrong");
                            continue;
                        }
                        
                        if result.is_object() {
                            // We have full transaction data
                            let hash = result.get("hash")
                                .and_then(|h| h.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            
                            let full_tx = FullTransaction {
                                hash: hash.clone(),
                                tx_data: result.clone(),
                                detection_time,
                                latency_ns,
                            };
                            
                            // Update stats
                            Self::update_stats(&stats, latency_ns).await;
                            
                            // Send transaction
                            match tx_sender.try_send(full_tx) {
                                Ok(_) => {
                                    let _new_size = queue_size.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                                    
                                    // Log ultra-fast detections
                                    if latency_ns < 1000 { // < 1μs
                                        info!("⚡ ULTRA-FAST: 0x{}... in {}ns", &hash[2..8], latency_ns);
                                    } else if latency_ns < 100_000 { // < 100μs
                                        debug!("✅ FAST: 0x{}... in {}μs", &hash[2..8], latency_ns / 1000);
                                    }
                                }
                                Err(_) => {
                                    warn!("FullTransactionIpcClient channel full, dropping transaction");
                                }
                            }
                            
                            // Progress reporting
                            if count % 100 == 0 {
                                let stats_snapshot = stats.read().await;
                                let current_q_size = queue_size.load(std::sync::atomic::Ordering::Relaxed);
                                info!("IPC: {} transactions, avg: {}ns ({}μs), queue: {}/50000", 
                                      count, stats_snapshot.avg_latency_ns, 
                                      stats_snapshot.avg_latency_ns / 1000, current_q_size);
                                
                                let sub_1ms_pct = if stats_snapshot.total_transactions > 0 {
                                    (stats_snapshot.sub_1ms_count as f64 / stats_snapshot.total_transactions as f64) * 100.0
                                } else { 0.0 };
                                
                                info!("   Sub-1ms: {:.1}% | Sub-100μs: {} | Sub-10μs: {}", 
                                      sub_1ms_pct, stats_snapshot.sub_100us_count, stats_snapshot.sub_10us_count);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Update performance statistics
    async fn update_stats(stats: &Arc<RwLock<IpcClientStats>>, latency_ns: u64) {
        let mut stats_guard = stats.write().await;
        stats_guard.total_transactions += 1;
        stats_guard.latencies.push_back(latency_ns);
        stats_guard.sum_latencies += latency_ns;
        
        // Keep only last 1000 for memory efficiency
        if stats_guard.latencies.len() > 1000 {
            if let Some(old_latency) = stats_guard.latencies.pop_front() {
                stats_guard.sum_latencies = stats_guard.sum_latencies.saturating_sub(old_latency);
            }
        }
        
        // Update average
        stats_guard.avg_latency_ns = if !stats_guard.latencies.is_empty() {
            stats_guard.sum_latencies / stats_guard.latencies.len() as u64
        } else {
            0
        };
        
        // Update counters
        if latency_ns < 10_000 {     // 10μs
            stats_guard.sub_10us_count += 1;
        }
        if latency_ns < 100_000 {    // 100μs  
            stats_guard.sub_100us_count += 1;
        }
        if latency_ns < 1_000_000 {  // 1ms
            stats_guard.sub_1ms_count += 1;
        }
        
        // Update min/max
        match stats_guard.min_latency_ns {
            None => stats_guard.min_latency_ns = Some(latency_ns),
            Some(min) if latency_ns < min => stats_guard.min_latency_ns = Some(latency_ns),
            _ => {}
        }
        match stats_guard.max_latency_ns {
            None => stats_guard.max_latency_ns = Some(latency_ns),
            Some(max) if latency_ns > max => stats_guard.max_latency_ns = Some(latency_ns),
            _ => {}
        }
    }
    
    /// Get transactions with full data
    pub async fn get_full_transactions(&self, max_count: usize) -> Result<Vec<FullTransaction>> {
        let mut receiver = self.tx_receiver.lock().await;
        let mut transactions = Vec::with_capacity(max_count);
        
        // Wait for at least one transaction
        match tokio::time::timeout(Duration::from_millis(25), receiver.recv()).await {
            Ok(Some(tx)) => {
                transactions.push(tx);
                self.queue_size.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            },
            Ok(None) => return Err(eyre!("IPC channel closed")),
            Err(_) => return Ok(transactions), // Timeout
        }
        
        // Get more transactions without blocking
        while transactions.len() < max_count {
            match receiver.try_recv() {
                Ok(tx) => {
                    transactions.push(tx);
                    self.queue_size.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                },
                Err(_) => break,
            }
        }
        
        Ok(transactions)
    }
    
    /// Get performance statistics
    pub async fn get_stats(&self) -> IpcClientStats {
        self.stats.read().await.clone()
    }
    
    /// Check connection status
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }
}