use crate::mempool_processor::types::*;
use ethers::prelude::*;
use eyre::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, trace, info, warn};
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use hex::encode as hex_encode;
use url::Url;

/// How we fetch mempool transactions
#[derive(Clone, Copy, Debug)]
pub enum FetchMode {
    RpcBatch,    // existing fast path
    RpcSingle,   // existing non-batch path
    DevP2p,      // new – subscribe to dev-p2p TxPool events
}

/// Abstract trait for transaction sources
pub trait TransactionSource {
    async fn get_transactions(&self) -> Result<Vec<TransactionView>>;
    async fn get_stats(&self) -> Result<(usize, usize)>; // (pending, queued)
}

/// Optimized mempool fetcher using HTTP RPC calls with various optimizations
/// for better performance with a local Ethereum node
pub struct MempoolFetcher {
    provider: Provider<Http>,
    /// Cache for recently seen transactions to avoid duplicate processing
    tx_cache: Arc<Mutex<HashMap<String, Instant>>>,
    /// Maximum size of the transaction cache
    cache_capacity: usize,
    /// Whether to use batch requests for performance optimization
    use_batch_requests: bool,
    /// Maximum batch size for batch requests
    max_batch_size: usize,
    /// Connection timeout in milliseconds
    timeout_ms: u64,
    /// Fetch mode
    fetch_mode: FetchMode,
}

impl MempoolFetcher {
    pub fn new(http_rpc_url: &str) -> Result<Self> {
        // Set a shorter timeout for a local node
        let timeout = std::time::Duration::from_millis(2000); // 2 seconds default
        
        // Create HTTP connection
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()?;
        
        let url = Url::parse(http_rpc_url)?;
        let provider = Provider::new(Http::new_with_client(url, client));
        
        Ok(Self { 
            provider,
            tx_cache: Arc::new(Mutex::new(HashMap::with_capacity(5000))),
            cache_capacity: 5000,
            use_batch_requests: true,
            max_batch_size: 100,
            timeout_ms: 2000,
            fetch_mode: FetchMode::RpcBatch,  // default
        })
    }
    
    /// Create a new fetcher with custom settings
    pub fn with_options(
        http_rpc_url: &str, 
        cache_capacity: usize, 
        use_batch_requests: bool,
        max_batch_size: usize,
        timeout_ms: u64,
        fetch_mode: FetchMode,
    ) -> Result<Self> {
        // Set a custom timeout
        let timeout = std::time::Duration::from_millis(timeout_ms);
        
        // Create HTTP connection with the custom timeout
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()?;
        
        let url = Url::parse(http_rpc_url)?;
        let provider = Provider::new(Http::new_with_client(url, client));
        
        Ok(Self { 
            provider,
            tx_cache: Arc::new(Mutex::new(HashMap::with_capacity(cache_capacity))),
            cache_capacity,
            use_batch_requests,
            max_batch_size,
            timeout_ms,
            fetch_mode,
        })
    }
    
    /// Get the current fetch mode
    pub fn fetch_mode(&self) -> FetchMode {
        self.fetch_mode
    }
    
    /// Helper to parse a transaction from JSON format
    fn parse_transaction(tx_json: &Value) -> Option<TransactionView> {
        let hash = tx_json.get("hash")?.as_str()?;
        let from = tx_json.get("from")?.as_str()?;
        let to = tx_json.get("to").and_then(|v| v.as_str());
        let value_str = tx_json.get("value")?.as_str()?;
        let input_str = tx_json.get("input").and_then(|v| v.as_str());
        
        // Parse hash (removing 0x prefix)
        let hash = hash.strip_prefix("0x").unwrap_or(hash);
        let hash_bytes = hex::decode(hash).ok()?;
        
        // Parse addresses (removing 0x prefix)
        let from = from.strip_prefix("0x").unwrap_or(from);
        let from_bytes = hex::decode(from).ok()?;
        
        let to_bytes = if let Some(to) = to {
            let to = to.strip_prefix("0x").unwrap_or(to);
            Some(hex::decode(to).ok()?)
        } else {
            None
        };
        
        // Parse value
        let value_str = value_str.strip_prefix("0x").unwrap_or(value_str);
        let value = U256::from_str_radix(value_str, 16).ok()?;
        
        // Parse additional fields (optional)
        let gas_price = tx_json.get("gasPrice")
            .and_then(|v| v.as_str())
            .and_then(|s| s.strip_prefix("0x").map(|s_inner| s_inner.to_string()).or_else(|| Some(s.to_string())))
            .and_then(|s| U256::from_str_radix(&s, 16).ok());
            
        let gas_limit = tx_json.get("gas")
            .and_then(|v| v.as_str())
            .and_then(|s| s.strip_prefix("0x").map(|s_inner| s_inner.to_string()).or_else(|| Some(s.to_string())))
            .and_then(|s| U256::from_str_radix(&s, 16).ok());
            
        let nonce = tx_json.get("nonce")
            .and_then(|v| v.as_str())
            .and_then(|s| s.strip_prefix("0x").map(|s_inner| s_inner.to_string()).or_else(|| Some(s.to_string())))
            .and_then(|s| U256::from_str_radix(&s, 16).ok());

        // Parse input data
        let input_data_bytes = if let Some(input) = input_str {
            let input = input.strip_prefix("0x").unwrap_or(input);
            if input == "" || input == "0x" { // Handle empty input cases explicitly
                None
            } else {
                hex::decode(input).ok()
            }
        } else {
            None
        };
        
        Some(TransactionView {
            hash: hash_bytes,
            from: from_bytes,
            to: to_bytes,
            value,
            gas_price,
            gas_limit,
            nonce,
            input_data: input_data_bytes,
        })
    }
}

impl TransactionSource for MempoolFetcher {
    async fn get_transactions(&self) -> Result<Vec<TransactionView>> {
        // If batch requests are enabled, use the optimized method
        match self.fetch_mode {
            FetchMode::RpcBatch => return self.get_transactions_batch().await,
            FetchMode::RpcSingle => { /* fall through to existing non-batch code */ }
            FetchMode::DevP2p => return self.get_transactions_devp2p().await,
        }
        
        // Otherwise use the original implementation
        debug!("Requesting txpool_content...");
        let response: Value = self.provider.request("txpool_content", ()).await?;
        
        let mut transactions = Vec::new();
        
        // Process pending transactions
        if let Some(pending) = response.get("pending").and_then(|p| p.as_object()) {
            for (_address, nonce_map) in pending {
                if let Some(nonce_obj) = nonce_map.as_object() {
                    for (_, tx_value) in nonce_obj {
                        if let Some(tx) = Self::parse_transaction(tx_value) {
                            transactions.push(tx);
                        }
                    }
                }
            }
        }
        
        // Process queued transactions
        if let Some(queued) = response.get("queued").and_then(|q| q.as_object()) {
            for (_address, nonce_map) in queued {
                if let Some(nonce_obj) = nonce_map.as_object() {
                    for (_, tx_value) in nonce_obj {
                        if let Some(tx) = Self::parse_transaction(tx_value) {
                            transactions.push(tx);
                        }
                    }
                }
            }
        }
        
        Ok(transactions)
    }
    
    async fn get_stats(&self) -> Result<(usize, usize)> {
        debug!("Requesting txpool_status...");
        // Use the TxpoolStatus method to get quick stats
        let txpool: TxpoolStatus = self.provider.request("txpool_status", ()).await?;
        
        // Extract pending and queued counts
        let pending_count = txpool.pending.as_u64() as usize;
        let queued_count = txpool.queued.as_u64() as usize;
        
        Ok((pending_count, queued_count))
    }
}

impl MempoolFetcher {
    /// Clean up old entries from the transaction cache
    fn prune_tx_cache(&self) {
        if let Ok(mut cache) = self.tx_cache.lock() {
            // If cache is at capacity, remove the oldest 20% of entries
            if cache.len() >= self.cache_capacity {
                // Sort by timestamp (oldest first)
                let mut entries: Vec<_> = cache.iter().collect();
                entries.sort_by_key(|(_, timestamp)| *timestamp);
                
                // Determine how many to remove (20% of capacity)
                let remove_count = self.cache_capacity / 5;
                
                // Get the keys to remove
                let keys_to_remove: Vec<String> = entries
                    .iter()
                    .take(remove_count)
                    .map(|(k, _)| (*k).clone())
                    .collect();
                
                // Remove them from the cache
                for key in keys_to_remove {
                    cache.remove(&key);
                }
                
                debug!("Pruned {} old transactions from cache, new size: {}", remove_count, cache.len());
            }
        }
    }
    
    /// Get transactions using batch requests for better performance with local node
    async fn get_transactions_batch(&self) -> Result<Vec<TransactionView>> {
        // First get transaction hashes from txpool_content
        debug!("Requesting txpool_content...");
        let start = Instant::now();
        let response: Value = self.provider.request("txpool_content", ()).await?;
        let request_time = start.elapsed();
        debug!("txpool_content response received in {:?}", request_time);
        
        // Collect transaction hashes for batch request
        let mut tx_hashes = Vec::new();
        let mut seen_hashes = HashMap::new();
        
        // Process pending transactions first (they're more important)
        if let Some(pending) = response.get("pending").and_then(|p| p.as_object()) {
            for (_address, nonce_map) in pending {
                if let Some(nonce_obj) = nonce_map.as_object() {
                    for (_, tx_value) in nonce_obj {
                        if let Some(hash_str) = tx_value.get("hash").and_then(|h| h.as_str()) {
                            // Normalize hash
                            let hash = hash_str.strip_prefix("0x").unwrap_or(hash_str).to_lowercase();
                            
                            // Skip if it's a duplicate or in our cache
                            if !seen_hashes.contains_key(&hash) {
                                if let Ok(cache) = self.tx_cache.lock() {
                                    if !cache.contains_key(&hash) {
                                        if let Ok(hash_bytes) = H256::from_str(&format!("0x{}", hash)) {
                                            tx_hashes.push(hash_bytes);
                                            seen_hashes.insert(hash, true);
                                            
                                            // Check if we reached batch size limit
                                            if tx_hashes.len() >= self.max_batch_size {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Check if we reached batch size limit
                    if tx_hashes.len() >= self.max_batch_size {
                        break;
                    }
                }
            }
        }
        
        // If we still have room, add queued transactions
        if tx_hashes.len() < self.max_batch_size {
            if let Some(queued) = response.get("queued").and_then(|q| q.as_object()) {
                for (_address, nonce_map) in queued {
                    if let Some(nonce_obj) = nonce_map.as_object() {
                        for (_, tx_value) in nonce_obj {
                            if let Some(hash_str) = tx_value.get("hash").and_then(|h| h.as_str()) {
                                // Normalize hash
                                let hash = hash_str.strip_prefix("0x").unwrap_or(hash_str).to_lowercase();
                                
                                // Skip if it's a duplicate or in our cache
                                if !seen_hashes.contains_key(&hash) {
                                    if let Ok(cache) = self.tx_cache.lock() {
                                        if !cache.contains_key(&hash) {
                                            if let Ok(hash_bytes) = H256::from_str(&format!("0x{}", hash)) {
                                                tx_hashes.push(hash_bytes);
                                                seen_hashes.insert(hash, true);
                                                
                                                // Check if we reached batch size limit
                                                if tx_hashes.len() >= self.max_batch_size {
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Check if we reached batch size limit
                        if tx_hashes.len() >= self.max_batch_size {
                            break;
                        }
                    }
                }
            }
        }
        
        debug!("Collected {} transaction hashes for batch request", tx_hashes.len());
        
        // If we have no transactions, return empty list
        if tx_hashes.is_empty() {
            return Ok(Vec::new());
        }
        
        // Create batched requests for transaction details
        let mut transactions = Vec::with_capacity(tx_hashes.len());
        
        // Limit batch size to avoid overloading node
        for chunk in tx_hashes.chunks(50) {
            let batch_start = Instant::now();
            
            let mut batch = Vec::with_capacity(chunk.len());
            for &hash in chunk {
                batch.push(self.provider.get_transaction(hash));
            }
            
            // Execute batch request
            let results = futures::future::join_all(batch).await;
            let batch_time = batch_start.elapsed();
            
            debug!("Batch of {} transaction details received in {:?}", chunk.len(), batch_time);
            
            // Process results
            for result in results {
                match result {
                    Ok(Some(tx)) => {
                        // Convert to our format
                        let from = tx.from;
                        let hash_bytes = tx.hash.as_bytes().to_vec();
                        let from_bytes = from.as_bytes().to_vec();
                        let to_bytes = tx.to.map(|to| to.as_bytes().to_vec());
                        let input_data = Some(tx.input.to_vec());
                        
                        // Add to transaction cache
                        if let Ok(mut cache) = self.tx_cache.lock() {
                            let hash_hex = hex_encode(&hash_bytes);
                            cache.insert(hash_hex, Instant::now());
                        }
                        
                        let tx_view = TransactionView {
                            hash: hash_bytes,
                            from: from_bytes,
                            to: to_bytes,
                            value: tx.value,
                            gas_price: tx.gas_price,
                            gas_limit: Some(tx.gas),
                            nonce: Some(tx.nonce),
                            input_data,
                        };
                        
                        transactions.push(tx_view);
                    },
                    Ok(None) => {
                        // Transaction might have been removed from mempool
                        trace!("Transaction not found in mempool");
                    },
                    Err(e) => {
                        warn!("Error fetching transaction: {}", e);
                    }
                }
            }
        }
        
        // Periodically clean up the cache
        self.prune_tx_cache();
        
        debug!("Successfully fetched {} transactions", transactions.len());
        Ok(transactions)
    }
    
    /// Get transactions using dev-p2p connection to transaction pool
    async fn get_transactions_devp2p(&self) -> Result<Vec<TransactionView>> {
        // Note: This is a placeholder implementation since reth's TxPool features might not be available
        // In a real implementation, we would connect to the reth node via IPC and listen for transaction events
        
        debug!("DevP2p transaction fetching not fully implemented yet");
        debug!("Using a simplified implementation that fetches transactions through RPC for now");
        
        // For now, just delegate to the RPC batch method
        debug!("Falling back to RPC batch method");
        self.get_transactions_batch().await
    }
}

/// Mock transaction source for testing
pub struct MockTransactionSource {
    pub transactions: Vec<TransactionView>,
    pub pending_count: usize,
    pub queued_count: usize,
}

impl MockTransactionSource {
    pub fn new() -> Self {
        Self {
            transactions: Vec::new(),
            pending_count: 0,
            queued_count: 0,
        }
    }
    
    /// Add a simulated high-value transaction
    pub fn add_high_value_tx(&mut self, from: &str, to: &str, value_eth: f64) {
        let from_bytes = hex::decode(from.strip_prefix("0x").unwrap_or(from)).unwrap_or_default();
        let to_bytes = hex::decode(to.strip_prefix("0x").unwrap_or(to)).unwrap_or_default();
        let hash_bytes = vec![0u8; 32]; // Dummy hash
        
        // Convert ETH to wei
        let wei_value = (value_eth * 1e18) as u64;
        
        self.transactions.push(TransactionView {
            hash: hash_bytes,
            from: from_bytes,
            to: Some(to_bytes),
            value: U256::from(wei_value),
            gas_price: Some(U256::from(20_000_000_000u64)), // 20 gwei
            gas_limit: Some(U256::from(21000u64)),          // Standard transfer
            nonce: Some(U256::from(0u64)),
            input_data: None, // Mock transactions are simple transfers
        });
        
        self.pending_count += 1;
    }
}

impl TransactionSource for MockTransactionSource {
    async fn get_transactions(&self) -> Result<Vec<TransactionView>> {
        Ok(self.transactions.clone())
    }
    
    async fn get_stats(&self) -> Result<(usize, usize)> {
        Ok((self.pending_count, self.queued_count))
    }
} 