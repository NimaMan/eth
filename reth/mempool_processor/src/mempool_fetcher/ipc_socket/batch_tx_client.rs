/// Batch Transaction IPC Client - Optimized for Speed
/// 
/// This client optimizes transaction fetching by:
/// 1. Batching multiple transaction requests together
/// 2. Using multiple parallel IPC connections
/// 3. Processing transactions in batches rather than one-by-one
/// 
/// NO CHANGES to detection logic - only makes data fetching faster

use std::time::Instant;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncBufReadExt, BufReader};
use serde_json::{Value, json};
use tracing::{info, debug, error, warn};
use eyre::{Result, eyre};
use ethers::types::{Transaction, H256};
use futures::future::join_all;

use crate::mempool_fetcher::types::TransactionView;
use crate::mempool_fetcher::ipc_socket::full_tx_client::FullIpcTransaction;

/// Batch configuration for optimal performance
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Number of transactions to batch together
    pub batch_size: usize,
    /// Maximum wait time before processing incomplete batch (ms)
    pub batch_timeout_ms: u64,
    /// Number of parallel IPC connections
    pub connection_pool_size: usize,
    /// Buffer size for pending transaction hashes
    pub buffer_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 20,           // Process 20 transactions at once
            batch_timeout_ms: 50,     // Wait max 50ms to fill batch
            connection_pool_size: 3,  // Use 3 parallel connections
            buffer_size: 1000,        // Buffer up to 1000 pending hashes
        }
    }
}

/// Batch transaction processor with multiple IPC connections
pub struct BatchTxIpcClient {
    /// IPC socket path
    socket_path: String,
    
    /// Batch configuration
    config: BatchConfig,
    
    /// Transaction output channel
    tx_sender: mpsc::Sender<Vec<FullIpcTransaction>>,
    
    /// Transaction receiver for consumers
    tx_receiver: Arc<Mutex<mpsc::Receiver<Vec<FullIpcTransaction>>>>,
    
    /// Hash buffer for batching
    hash_buffer: Arc<Mutex<Vec<(String, Instant)>>>,
    
    /// Performance statistics
    stats: Arc<RwLock<BatchStats>>,
    
    /// Connection pool
    connection_pool: Arc<Mutex<Vec<IpcConnection>>>,
}

/// Performance statistics for batch processing
#[derive(Debug, Default, Clone)]
pub struct BatchStats {
    pub total_transactions: u64,
    pub total_batches: u64,
    pub avg_batch_size: f64,
    pub avg_latency_us: u64,
    pub min_latency_us: Option<u64>,
    pub max_latency_us: Option<u64>,
    pub sub_1ms_count: u64,
    pub sub_10ms_count: u64,
    pub batch_efficiency: f64, // percentage of full batches
}

/// Individual IPC connection in the pool
struct IpcConnection {
    id: usize,
    stream: BufReader<UnixStream>,
    is_busy: bool,
    requests_sent: u64,
}

impl BatchTxIpcClient {
    /// Create new batch IPC client
    pub fn new(socket_path: Option<&str>, config: Option<BatchConfig>) -> Result<Self> {
        let socket_path = socket_path.unwrap_or("/tmp/reth.ipc").to_string();
        let config = config.unwrap_or_default();
        let (tx_sender, tx_receiver) = mpsc::channel(1000);
        
        Ok(Self {
            socket_path,
            config,
            tx_sender,
            tx_receiver: Arc::new(Mutex::new(tx_receiver)),
            hash_buffer: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(RwLock::new(BatchStats::default())),
            connection_pool: Arc::new(Mutex::new(Vec::new())),
        })
    }
    
    /// Start batch processing with connection pool
    pub async fn start_batch_processing(&self) -> Result<()> {
        info!("🚀 Starting batch transaction processing");
        info!("   Batch size: {}", self.config.batch_size);
        info!("   Timeout: {}ms", self.config.batch_timeout_ms);
        info!("   Connections: {}", self.config.connection_pool_size);
        
        // Initialize connection pool
        self.initialize_connection_pool().await?;
        
        // Start hash collection from existing IPC subscription
        self.start_hash_collection().await?;
        
        // Start batch processor
        self.start_batch_processor().await?;
        
        Ok(())
    }
    
    /// Initialize multiple IPC connections
    async fn initialize_connection_pool(&self) -> Result<()> {
        let mut pool = self.connection_pool.lock().await;
        
        for i in 0..self.config.connection_pool_size {
            let stream = UnixStream::connect(&self.socket_path).await
                .map_err(|e| eyre!("Failed to connect IPC connection {}: {}", i, e))?;
            
            let connection = IpcConnection {
                id: i,
                stream: BufReader::new(stream),
                is_busy: false,
                requests_sent: 0,
            };
            
            pool.push(connection);
            debug!("✅ IPC connection {} established", i);
        }
        
        info!("✅ Connection pool initialized with {} connections", pool.len());
        Ok(())
    }
    
    /// Start collecting transaction hashes from subscription
    async fn start_hash_collection(&self) -> Result<()> {
        let socket_path = self.socket_path.clone();
        let hash_buffer = self.hash_buffer.clone();
        
        tokio::spawn(async move {
            if let Err(e) = Self::collect_transaction_hashes(socket_path, hash_buffer).await {
                error!("Hash collection failed: {}", e);
            }
        });
        
        Ok(())
    }
    
    /// Collect transaction hashes from subscription (same as existing logic)
    async fn collect_transaction_hashes(
        socket_path: String,
        hash_buffer: Arc<Mutex<Vec<(String, Instant)>>>
    ) -> Result<()> {
        let stream = UnixStream::connect(socket_path).await?;
        let mut stream = BufReader::new(stream);
        
        // Subscribe to pending transactions (hashes only for now)
        let subscribe_request = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["newPendingTransactions"],
            "id": 1
        });
        
        let request_str = format!("{}\n", subscribe_request);
        stream.get_mut().write_all(request_str.as_bytes()).await?;
        
        // Read subscription response
        let mut response_line = String::new();
        stream.read_line(&mut response_line).await?;
        
        info!("📡 Hash collection subscription established");
        
        // Collect hashes
        let mut line = String::new();
        loop {
            line.clear();
            match stream.read_line(&mut line).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if let Ok(message) = serde_json::from_str::<Value>(&line) {
                        if let Some(params) = message.get("params") {
                            if let Some(result) = params.get("result") {
                                if let Some(tx_hash) = result.as_str() {
                                    let detection_time = Instant::now();
                                    
                                    let mut buffer = hash_buffer.lock().await;
                                    buffer.push((tx_hash.to_string(), detection_time));
                                    
                                    // Prevent buffer overflow
                                    if buffer.len() > 2000 {
                                        buffer.drain(0..500); // Remove oldest 500
                                        warn!("Hash buffer overflow - drained old entries");
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading hash subscription: {}", e);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Start the batch processor
    async fn start_batch_processor(&self) -> Result<()> {
        let hash_buffer = self.hash_buffer.clone();
        let connection_pool = self.connection_pool.clone();
        let tx_sender = self.tx_sender.clone();
        let stats = self.stats.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut batch_interval = tokio::time::interval(
                tokio::time::Duration::from_millis(config.batch_timeout_ms)
            );
            
            loop {
                batch_interval.tick().await;
                
                // Collect batch of hashes
                let batch = {
                    let mut buffer = hash_buffer.lock().await;
                    if buffer.is_empty() {
                        continue;
                    }
                    
                    let batch_size = std::cmp::min(config.batch_size, buffer.len());
                    buffer.drain(0..batch_size).collect::<Vec<_>>()
                };
                
                if !batch.is_empty() {
                    // Process batch in parallel using connection pool
                    if let Err(e) = Self::process_batch(
                        batch,
                        &connection_pool,
                        &tx_sender,
                        &stats
                    ).await {
                        error!("Batch processing failed: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Process a batch of transaction hashes
    async fn process_batch(
        batch: Vec<(String, Instant)>,
        connection_pool: &Arc<Mutex<Vec<IpcConnection>>>,
        tx_sender: &mpsc::Sender<Vec<FullIpcTransaction>>,
        stats: &Arc<RwLock<BatchStats>>
    ) -> Result<()> {
        let batch_start = Instant::now();
        let batch_size = batch.len();
        
        debug!("🔄 Processing batch of {} transactions", batch_size);
        
        // Split batch across available connections
        let chunks = Self::split_batch_for_connections(batch, connection_pool).await?;
        
        // Process chunks in parallel
        let mut tasks = Vec::new();
        for (chunk, connection_id) in chunks {
            let pool = connection_pool.clone();
            
            let task = tokio::spawn(async move {
                Self::fetch_chunk(chunk, connection_id, pool).await
            });
            
            tasks.push(task);
        }
        
        // Wait for all chunks to complete
        let results = join_all(tasks).await;
        
        // Collect all transactions
        let mut all_transactions = Vec::new();
        for result in results {
            match result {
                Ok(Ok(mut transactions)) => {
                    all_transactions.append(&mut transactions);
                }
                Ok(Err(e)) => {
                    error!("Chunk processing error: {}", e);
                }
                Err(e) => {
                    error!("Task join error: {}", e);
                }
            }
        }
        
        // Send batch result
        if !all_transactions.is_empty() {
            if let Err(e) = tx_sender.send(all_transactions.clone()).await {
                error!("Failed to send batch result: {}", e);
            }
        }
        
        // Update statistics
        let batch_latency = batch_start.elapsed().as_micros() as u64;
        Self::update_batch_stats(stats, batch_size, batch_latency, &all_transactions).await;
        
        debug!("✅ Batch completed: {} transactions in {}μs", 
               all_transactions.len(), batch_latency);
        
        Ok(())
    }
    
    /// Split batch across available connections
    async fn split_batch_for_connections(
        batch: Vec<(String, Instant)>,
        connection_pool: &Arc<Mutex<Vec<IpcConnection>>>
    ) -> Result<Vec<(Vec<(String, Instant)>, usize)>> {
        let pool = connection_pool.lock().await;
        let available_connections: Vec<usize> = pool
            .iter()
            .enumerate()
            .filter(|(_, conn)| !conn.is_busy)
            .map(|(i, _)| i)
            .collect();
        
        if available_connections.is_empty() {
            return Err(eyre!("No available connections"));
        }
        
        let chunk_size = (batch.len() + available_connections.len() - 1) / available_connections.len();
        let mut chunks = Vec::new();
        
        for (i, &connection_id) in available_connections.iter().enumerate() {
            let start = i * chunk_size;
            let end = std::cmp::min(start + chunk_size, batch.len());
            
            if start < batch.len() {
                let chunk = batch[start..end].to_vec();
                chunks.push((chunk, connection_id));
            }
        }
        
        Ok(chunks)
    }
    
    /// Fetch a chunk of transactions using specific connection
    async fn fetch_chunk(
        chunk: Vec<(String, Instant)>,
        connection_id: usize,
        connection_pool: Arc<Mutex<Vec<IpcConnection>>>
    ) -> Result<Vec<FullIpcTransaction>> {
        let mut transactions = Vec::new();
        
        // Create batch request for all hashes in chunk
        let mut batch_request = Vec::new();
        for (i, (hash, _)) in chunk.iter().enumerate() {
            let request = json!({
                "jsonrpc": "2.0",
                "method": "eth_getTransactionByHash",
                "params": [hash],
                "id": connection_id * 1000 + i
            });
            batch_request.push(request);
        }
        
        // Send batch request
        {
            let mut pool = connection_pool.lock().await;
            if let Some(connection) = pool.get_mut(connection_id) {
                connection.is_busy = true;
                
                // Send all requests in the batch
                for request in &batch_request {
                    let request_str = format!("{}\n", request);
                    connection.stream.get_mut().write_all(request_str.as_bytes()).await?;
                }
                
                // Read responses
                for (request, (hash, detection_time)) in batch_request.iter().zip(chunk.iter()) {
                    let mut response_line = String::new();
                    match connection.stream.read_line(&mut response_line).await {
                        Ok(_) => {
                            if let Ok(response) = serde_json::from_str::<Value>(&response_line) {
                                if let Some(result) = response.get("result") {
                                    if !result.is_null() {
                                        if let Ok(transaction) = serde_json::from_value::<Transaction>(result.clone()) {
                                            let tx_view = TransactionView::from_ethers_transaction(&transaction);
                                            let latency_us = detection_time.elapsed().as_micros() as u64;
                                            
                                            let full_tx = FullIpcTransaction {
                                                hash: hash.clone(),
                                                transaction,
                                                tx_view,
                                                detection_time: *detection_time,
                                                latency_us,
                                            };
                                            
                                            transactions.push(full_tx);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error reading response for {}: {}", hash, e);
                        }
                    }
                }
                
                connection.is_busy = false;
                connection.requests_sent += batch_request.len() as u64;
            }
        }
        
        Ok(transactions)
    }
    
    /// Update batch statistics
    async fn update_batch_stats(
        stats: &Arc<RwLock<BatchStats>>,
        batch_size: usize,
        batch_latency_us: u64,
        transactions: &[FullIpcTransaction]
    ) {
        let mut stats_guard = stats.write().await;
        
        stats_guard.total_batches += 1;
        stats_guard.total_transactions += transactions.len() as u64;
        
        // Update batch efficiency
        let full_batches = if batch_size >= 15 { 1 } else { 0 }; // Consider 75% of batch_size as "full"
        stats_guard.batch_efficiency = 
            (full_batches as f64) / (stats_guard.total_batches as f64) * 100.0;
        
        // Update average batch size
        stats_guard.avg_batch_size = 
            (stats_guard.total_transactions as f64) / (stats_guard.total_batches as f64);
        
        // Update latency stats
        for tx in transactions {
            if tx.latency_us < 1000 {
                stats_guard.sub_1ms_count += 1;
            }
            if tx.latency_us < 10000 {
                stats_guard.sub_10ms_count += 1;
            }
            
            match stats_guard.min_latency_us {
                None => stats_guard.min_latency_us = Some(tx.latency_us),
                Some(min) if tx.latency_us < min => stats_guard.min_latency_us = Some(tx.latency_us),
                _ => {}
            }
            match stats_guard.max_latency_us {
                None => stats_guard.max_latency_us = Some(tx.latency_us),
                Some(max) if tx.latency_us > max => stats_guard.max_latency_us = Some(tx.latency_us),
                _ => {}
            }
        }
        
        // Calculate average latency
        if !transactions.is_empty() {
            let total_latency: u64 = transactions.iter().map(|tx| tx.latency_us).sum();
            stats_guard.avg_latency_us = total_latency / transactions.len() as u64;
        }
    }
    
    /// Get batched transactions for processing
    pub async fn get_transaction_batches(&self, max_batches: usize) -> Result<Vec<Vec<FullIpcTransaction>>> {
        let mut receiver = self.tx_receiver.lock().await;
        let mut batches = Vec::new();
        
        while batches.len() < max_batches {
            match receiver.try_recv() {
                Ok(batch) => batches.push(batch),
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    return Err(eyre!("Batch channel disconnected"));
                }
            }
        }
        
        Ok(batches)
    }
    
    /// Get performance statistics
    pub async fn get_stats(&self) -> BatchStats {
        self.stats.read().await.clone()
    }
    
    /// Get configuration
    pub fn get_config(&self) -> &BatchConfig {
        &self.config
    }
}