use std::time::Instant;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::net::UnixStream;
use tokio::time::{timeout, Duration};
use serde_json::{Value, json};
use tracing::{info, error, warn};
use eyre::{Result, eyre};
use hex;

/// Transaction received from mempool via IPC
#[derive(Debug, Clone)]
pub struct MempoolTransaction {
    pub hash: String,
    pub data: Value,  // JSON transaction data (equivalent to FullTransaction.tx_data)
    pub detection_ns: u64,
    // Add missing fields for compatibility with FullTransaction
    pub detection_time: Instant,  // When we detected it
    pub latency_ns: u64,  // Detection latency in nanoseconds (alias for detection_ns)
    // Pre-parsed fields for fast access
    pub from: Vec<u8>,
    pub to: Option<Vec<u8>>,
    pub input: Vec<u8>,
    pub value: ethers::types::U256,
    pub gas_price: Option<ethers::types::U256>,
    pub functions: Vec<String>, // ["transfer", "liquidity_removal", etc.]
    pub function_category: Option<crate::function_detector::CreatorFunctionType>, // Function category from detector
}

pub struct MempoolFetcherIPCClient {
    socket_path: String,
    tx_sender: mpsc::Sender<MempoolTransaction>,
    tx_receiver: Arc<Mutex<mpsc::Receiver<MempoolTransaction>>>,
    stats: Arc<RwLock<Stats>>,
    queue_size: Arc<AtomicUsize>,
}

#[derive(Default, Clone)]
pub struct Stats {
    pub total: u64,
    pub sub_1ms: u64,
    pub sub_100us: u64,
    pub sub_10us: u64,
    pub queue_size: usize,
}

impl MempoolFetcherIPCClient {
    pub fn new(socket_path: Option<&str>) -> Result<Self> {
        let socket_path = socket_path.unwrap_or("/tmp/reth.ipc").to_string();
        let (tx_sender, tx_receiver) = mpsc::channel(50000);
        
        Ok(Self {
            socket_path,
            tx_sender,
            tx_receiver: Arc::new(Mutex::new(tx_receiver)),
            stats: Arc::new(RwLock::new(Stats::default())),
            queue_size: Arc::new(AtomicUsize::new(0)),
        })
    }
    
    pub async fn start(&self) -> Result<()> {
        // Add timeout to connection attempt (5 seconds)
        let stream = match timeout(
            Duration::from_secs(5),
            UnixStream::connect(&self.socket_path)
        ).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => return Err(eyre!("Failed to connect to IPC socket: {}", e)),
            Err(_) => return Err(eyre!("IPC connection timed out after 5 seconds")),
        };
        
        // Subscribe with correct parameters
        let subscribe = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["newPendingTransactions", true],
            "id": 1
        });
        
        let socket = stream.into_std()?;
        socket.set_nonblocking(true)?;
        
        // Send subscription
        use std::io::Write;
        let mut socket = socket;
        socket.write_all(format!("{}\n", subscribe).as_bytes())?;
        socket.flush()?;
        
        // Quick check for subscription response
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut buf = vec![0u8; 4096];
        match socket.read(&mut buf) {
            Ok(n) => {
                let response = std::str::from_utf8(&buf[..n])?;
                if !response.contains("result") {
                    return Err(eyre!("Subscription failed: {}", response));
                }
                info!("Subscription active");
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                info!("No subscription response yet");
            }
            Err(e) => return Err(e.into()),
        }
        
        // Convert back to async
        let stream = UnixStream::from_std(socket)?;
        
        // Start monitoring with ultra-fast detection
        let tx_sender = self.tx_sender.clone();
        let stats = self.stats.clone();
        let queue_size = self.queue_size.clone();
        
        tokio::spawn(async move {
            if let Err(e) = Self::monitor_nonblocking(stream, tx_sender, stats, queue_size).await {
                error!("Monitor error: {}", e);
            }
        });
        
        Ok(())
    }
    
    async fn monitor_nonblocking(
        stream: UnixStream,
        tx_sender: mpsc::Sender<MempoolTransaction>,
        stats: Arc<RwLock<Stats>>,
        queue_size: Arc<AtomicUsize>,
    ) -> Result<()> {
        
        
        let mut buffer = vec![0u8; 65536]; // 64KB
        let mut pending = Vec::with_capacity(1024 * 1024); // 1MB
        const MAX_PENDING_SIZE: usize = 10 * 1024 * 1024; // 10MB limit
        
        loop {
            // Try to read data with minimal blocking
            let detect_start = Instant::now();
            
            match stream.try_read(&mut buffer) {
                Ok(n) => {
                    if n == 0 {
                        break; // Connection closed
                    }
                    
                    // ULTRA-FAST DETECTION!
                    let detection_ns = detect_start.elapsed().as_nanos() as u64;
                    
                    // Append data with bounds checking
                    if pending.len() + n > MAX_PENDING_SIZE {
                        warn!("Pending buffer too large ({} bytes), clearing to prevent memory exhaustion", pending.len());
                        pending.clear();
                        continue;
                    }
                    pending.extend_from_slice(&buffer[..n]);
                    
                    // Process complete JSON lines
                    while let Some(pos) = pending.iter().position(|&b| b == b'\n') {
                        let line = pending.drain(..=pos).collect::<Vec<u8>>();
                        
                        // Quick parse
                        if let Ok(json_str) = std::str::from_utf8(&line) {
                            if let Ok(notification) = serde_json::from_str::<Value>(json_str) {
                                if let Some(params) = notification.get("params") {
                                    if let Some(result) = params.get("result") {
                                        if result.is_object() {
                                            let hash = result.get("hash")
                                                .and_then(|h| h.as_str())
                                                .unwrap_or("unknown")
                                                .to_string();
                                            
                                            // Update stats
                                            {
                                                let mut stats = stats.write().await;
                                                stats.total += 1;
                                                if detection_ns < 1_000_000 { stats.sub_1ms += 1; }
                                                if detection_ns < 100_000 { stats.sub_100us += 1; }
                                                if detection_ns < 10_000 { stats.sub_10us += 1; }
                                            }
                                            
                                            
                                            // Pre-parse transaction fields
                                            let from = result.get("from")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| hex::decode(s.trim_start_matches("0x")).ok())
                                                .unwrap_or_default();
                                                
                                            let to = result.get("to")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| hex::decode(s.trim_start_matches("0x")).ok());
                                                
                                            let input = result.get("input")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| hex::decode(s.trim_start_matches("0x")).ok())
                                                .unwrap_or_default();
                                                
                                            let value = result.get("value")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| ethers::types::U256::from_str_radix(s.trim_start_matches("0x"), 16).ok())
                                                .unwrap_or_default();
                                                
                                            let gas_price = result.get("gasPrice")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| ethers::types::U256::from_str_radix(s.trim_start_matches("0x"), 16).ok());
                                            
                                            let detection_time = Instant::now();
                                            let tx = MempoolTransaction {
                                                hash,
                                                data: result.clone(),
                                                detection_ns,
                                                detection_time,
                                                latency_ns: detection_ns, // Same value as detection_ns for compatibility
                                                from,
                                                to,
                                                input,
                                                value,
                                                gas_price,
                                                functions: Vec::new(), // Will be populated by function detector
                                                function_category: None, // Will be populated by function detector
                                            };
                                            
                                            match tx_sender.try_send(tx) {
                                                Ok(_) => {
                                                    queue_size.fetch_add(1, Ordering::Relaxed);
                                                }
                                                Err(e) => {
                                                    warn!("Channel full, dropping transaction: {}", e);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Prevent unbounded growth
                    if pending.capacity() > 10_000_000 {
                        pending = Vec::with_capacity(1024 * 1024);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data ready, yield briefly
                    tokio::time::sleep(std::time::Duration::from_micros(10)).await;
                }
                Err(e) => {
                    error!("Read error: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn get_transactions(&self, max: usize) -> Result<Vec<MempoolTransaction>> {
        let mut receiver = self.tx_receiver.lock().await;
        let mut txs = Vec::with_capacity(max);
        
        // Get first transaction (with timeout)
        match tokio::time::timeout(
            std::time::Duration::from_millis(25),
            receiver.recv()
        ).await {
            Ok(Some(tx)) => {
                txs.push(tx);
                self.queue_size.fetch_sub(1, Ordering::Relaxed);
            },
            Ok(None) => return Err(eyre!("Channel closed")),
            Err(_) => return Ok(txs), // Timeout
        }
        
        // Get more without blocking
        while txs.len() < max {
            match receiver.try_recv() {
                Ok(tx) => {
                    txs.push(tx);
                    self.queue_size.fetch_sub(1, Ordering::Relaxed);
                },
                Err(_) => break,
            }
        }
        
        // Update stats with current queue size
        let current_queue_size = self.queue_size.load(Ordering::Relaxed);
        self.stats.write().await.queue_size = current_queue_size;
        
        Ok(txs)
    }
    
    /// Get transactions instantly without any waiting - for ultra-low latency
    pub async fn get_transactions_instant(&self, max: usize) -> Vec<MempoolTransaction> {
        let mut receiver = self.tx_receiver.lock().await;
        let mut txs = Vec::with_capacity(max);
        
        // No waiting - just drain what's available immediately
        while txs.len() < max {
            match receiver.try_recv() {
                Ok(tx) => {
                    txs.push(tx);
                    self.queue_size.fetch_sub(1, Ordering::Relaxed);
                },
                Err(_) => break,
            }
        }
        
        // Update stats with current queue size
        let current_queue_size = self.queue_size.load(Ordering::Relaxed);
        self.stats.write().await.queue_size = current_queue_size;
        
        txs
    }
    
    pub async fn get_stats(&self) -> Stats {
        self.stats.read().await.clone()
    }
}

use std::io::Read;

impl std::fmt::Debug for MempoolFetcherIPCClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MempoolFetcherIPCClient")
            .field("socket_path", &self.socket_path)
            .finish()
    }
}
