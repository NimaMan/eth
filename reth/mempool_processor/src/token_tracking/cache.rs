// token_tracking/cache.rs
//
// Thread-safe cache for storing and retrieving pool state and token creator information.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::types::{PoolState, PoolUpdate, TokenCreator, TokenCreatorState};

/// Thread-safe cache for storing the latest pool ETH levels and metadata.
#[derive(Debug, Clone)]
pub struct PoolStateCache {
    /// Internal storage using RwLock for thread-safe concurrent access
    /// Maps pool addresses (as hex strings) to their current state
    storage: Arc<RwLock<HashMap<String, PoolState>>>,
    
    /// Threshold ETH value for scam detection
    eth_threshold: f64,
    
    /// Maximum number of pools to keep in cache
    max_pools: usize,
}

impl PoolStateCache {
    /// Create a new pool state cache with the specified ETH threshold
    pub fn new(eth_threshold: f64) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            eth_threshold,
            max_pools: 100_000, // 100K pools max
        }
    }
    
    /// Update the cache with a new PoolUpdatesMessage
    /// Returns a Vec of pool addresses that were updated
    pub async fn update_pools<'a, I>(&self, pool_updates: I) -> Vec<String> 
    where
        I: IntoIterator<Item = (&'a String, &'a PoolUpdate)>,
    {
        let mut updated_addresses = Vec::new();
        
        // Lock for writing (exclusive access)
        let mut storage = self.storage.write().await;
        
        // Process each pool update
        for (address, update) in pool_updates {
            let pool_state = PoolState::from(update.clone());
            storage.insert(address.clone(), pool_state);
            updated_addresses.push(address.clone());
        }
        
        // Check cache size and evict old entries if necessary
        // Calculate excess before any modifications to ensure consistency
        let current_size = storage.len();
        if current_size > self.max_pools {
            let excess = current_size.saturating_sub(self.max_pools);
            if excess > 0 {
                warn!("Pool cache exceeded limit ({} pools), evicting {} oldest entries", current_size, excess);
                
                // Convert to Vec and sort by age (oldest first) - do this atomically
                let mut entries: Vec<_> = storage.iter()
                    .map(|(addr, state)| (addr.clone(), state.received_at))
                    .collect();
                entries.sort_by_key(|(_, received_at)| *received_at);
                
                // Collect addresses to remove first, then remove them
                let addresses_to_remove: Vec<_> = entries.into_iter()
                    .take(excess)
                    .map(|(addr, _)| addr)
                    .collect();
                
                // Now remove them all at once
                for addr in addresses_to_remove {
                    storage.remove(&addr);
                }
            }
        }
        
        // Log the update
        if !updated_addresses.is_empty() {
            debug!("Updated {} pools in state cache (total: {})", updated_addresses.len(), storage.len());
        }
        
        updated_addresses
    }
    
    /// Get the current state of a specific pool
    pub async fn get_pool(&self, address: &str) -> Option<PoolState> {
        // Lock for reading (shared access)
        let storage = self.storage.read().await;
        
        // Get and clone the pool state (if it exists)
        storage.get(address).cloned()
    }
    
    /// Get all pools in the cache
    pub async fn get_all_pools(&self) -> HashMap<String, PoolState> {
        // Lock for reading (shared access)
        let storage = self.storage.read().await;
        
        // Clone the entire map for caller's use
        storage.clone()
    }
    
    /// Get the current ETH threshold used for scam detection
    pub fn get_eth_threshold(&self) -> f64 {
        self.eth_threshold
    }
    
    /// Update the ETH threshold used for scam detection
    pub fn set_eth_threshold(&mut self, threshold: f64) {
        self.eth_threshold = threshold;
        info!("Updated ETH threshold to {}", threshold);
    }
    
    /// Get the number of pools currently in the cache
    pub async fn get_pool_count(&self) -> usize {
        // Lock for reading (shared access)
        let storage = self.storage.read().await;
        
        storage.len()
    }
}

/// Thread-safe cache for storing token creator information and risk metrics.
#[derive(Debug, Clone)]
pub struct TokenCreatorCache {
    /// Internal storage mapping token addresses to creator information
    /// Keys are token contract addresses (as hex strings)
    /// Values are creator state information
    token_storage: Arc<RwLock<HashMap<String, TokenCreatorState>>>,
    
    /// Internal storage mapping creator addresses to their tokens
    /// Keys are creator addresses (as hex strings)
    /// Values are sets of token addresses created by this creator
    creator_storage: Arc<RwLock<HashMap<String, Vec<String>>>>,
    
    /// Maximum number of token creators to keep in cache
    max_creators: usize,
}

impl TokenCreatorCache {
    /// Create a new token creator cache
    pub fn new() -> Self {
        Self {
            token_storage: Arc::new(RwLock::new(HashMap::new())),
            creator_storage: Arc::new(RwLock::new(HashMap::new())),
            max_creators: 50_000, // 50K creators max
        }
    }
    
    /// Update the cache with token creator information
    /// Returns the number of creators updated
    pub async fn update_creators<'a, I>(&self, creator_updates: I) -> usize 
    where
        I: IntoIterator<Item = (&'a String, &'a TokenCreator)>,
    {
        let mut updated_count = 0;
        
        // Lock both storages for writing
        let mut token_storage = self.token_storage.write().await;
        let mut creator_storage = self.creator_storage.write().await;
        
        // Process each creator update
        for (token_address, creator) in creator_updates {
            // Store token -> creator mapping
            let creator_state = TokenCreatorState::from(creator.clone());
            token_storage.insert(token_address.clone(), creator_state);
            
            // Store creator -> tokens mapping
            creator_storage
                .entry(creator.creator_address.clone())
                .or_insert_with(Vec::new)
                .push(token_address.clone());
            
            updated_count += 1;
        }
        
        // Check cache size and evict old entries if necessary
        if token_storage.len() > self.max_creators {
            let excess = token_storage.len() - self.max_creators;
            warn!("Creator cache exceeded limit ({} creators), evicting {} oldest entries", token_storage.len(), excess);
            
            // Convert to Vec and sort by age (oldest first)
            let mut entries: Vec<_> = token_storage.iter()
                .map(|(addr, state)| (addr.clone(), state.received_at))
                .collect();
            entries.sort_by_key(|(_, received_at)| *received_at);
            
            // Remove oldest entries
            for (addr, _) in entries.into_iter().take(excess) {
                token_storage.remove(&addr);
            }
        }
        
        if updated_count > 0 {
            debug!("Updated {} token creators in cache (total: {})", updated_count, token_storage.len());
        }
        
        updated_count
    }
    
    /// Get creator information for a specific token
    pub async fn get_token_creator(&self, token_address: &str) -> Option<TokenCreatorState> {
        let storage = self.token_storage.read().await;
        
        storage.get(token_address).cloned()
    }
    
    /// Get all tokens created by a specific creator address
    pub async fn get_creator_tokens(&self, creator_address: &str) -> Vec<String> {
        let storage = self.creator_storage.read().await;
        
        storage.get(creator_address).cloned().unwrap_or_default()
    }
    
    /// Get creators who use private mempool
    pub async fn get_private_mempool_creators(&self) -> Vec<TokenCreatorState> {
        let storage = self.token_storage.read().await;
        
        storage
            .values()
            .filter(|creator_state| creator_state.creator.uses_private_mempool)
            .cloned()
            .collect()
    }
    
    /// Get all token creators in the cache
    pub async fn get_all_creators(&self) -> HashMap<String, TokenCreatorState> {
        let storage = self.token_storage.read().await;
        
        storage.clone()
    }
    
    /// Get the number of token creators currently in the cache
    pub async fn get_creator_count(&self) -> usize {
        let storage = self.token_storage.read().await;
        
        storage.len()
    }
    
    /// Check if a token's creator uses private mempool
    pub async fn is_token_creator_private(&self, token_address: &str) -> bool {
        if let Some(creator_state) = self.get_token_creator(token_address).await {
            creator_state.creator.uses_private_mempool
        } else {
            false
        }
    }
    
    /// Get token address by creator address (reverse lookup)
    pub async fn get_token_by_creator(&self, creator_address: &str) -> Option<String> {
        let storage = self.token_storage.read().await;
        
        // Search through all tokens to find one created by this address
        for (token_address, creator_state) in storage.iter() {
            if creator_state.creator.creator_address.eq_ignore_ascii_case(creator_address) {
                return Some(token_address.clone());
            }
        }
        
        None
    }
    
    /// Record an observed transaction from a creator
    pub async fn record_creator_transaction(&self, creator_address: &str, tx_hash: &str, 
                                     function_name: &str, seen_in_mempool: bool) {
        let mut storage = self.token_storage.write().await;
        
        // Find the token created by this address
        let token_address = storage.iter()
            .find(|(_, state)| state.creator.creator_address.eq_ignore_ascii_case(creator_address))
            .map(|(addr, _)| addr.clone());
            
        if let Some(token_addr) = token_address {
            if let Some(creator_state) = storage.get_mut(&token_addr) {
                // Add the observed transaction
                creator_state.observed_transactions.push(
                    super::types::ObservedTransaction {
                        tx_hash: tx_hash.to_string(),
                        block_number: None, // Will be set when mined
                        function_name: function_name.to_string(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        seen_in_mempool,
                    }
                );
                
                // Check if we have enough data to determine mempool usage
                if creator_state.observed_transactions.len() >= 3 && creator_state.mempool_usage_determined.is_none() {
                    let total_txs = creator_state.observed_transactions.len();
                    let mempool_visible = creator_state.observed_transactions.iter()
                        .filter(|tx| tx.seen_in_mempool)
                        .count();
                    
                    let visibility_ratio = mempool_visible as f64 / total_txs as f64;
                    
                    // Determine mempool usage pattern
                    if visibility_ratio >= 0.7 {
                        creator_state.mempool_usage_determined = Some(false); // Public mempool user
                        creator_state.creator.uses_private_mempool = false;
                        info!("Creator {} determined to be PUBLIC mempool user ({}% visible)", 
                              creator_address, (visibility_ratio * 100.0) as u32);
                    } else if visibility_ratio <= 0.3 {
                        creator_state.mempool_usage_determined = Some(true); // Private mempool user
                        creator_state.creator.uses_private_mempool = true;
                        warn!("Creator {} determined to be PRIVATE mempool user ({}% visible)", 
                              creator_address, (visibility_ratio * 100.0) as u32);
                    }
                    // Between 30-70% = mixed usage, keep as None
                }
            }
        }
    }
    
    /// Mark a transaction as mined with its block number
    pub async fn mark_transaction_mined(&self, tx_hash: &str, block_number: u64) {
        let mut storage = self.token_storage.write().await;
        
        // Update block number for this transaction across all creators
        for creator_state in storage.values_mut() {
            for tx in &mut creator_state.observed_transactions {
                if tx.tx_hash == tx_hash {
                    tx.block_number = Some(block_number);
                }
            }
        }
    }
    
    /// Remove stale creator entries older than the specified duration
    pub async fn cleanup_stale_entries(&self, max_age: std::time::Duration) -> usize {
        let mut token_storage = self.token_storage.write().await;
        
        let initial_count = token_storage.len();
        token_storage.retain(|_, creator_state| !creator_state.is_stale(max_age));
        let removed_count = initial_count - token_storage.len();
        
        if removed_count > 0 {
            info!("Cleaned up {} stale token creator entries", removed_count);
        }
        
        removed_count
    }
}

impl Default for TokenCreatorCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined cache for both pool states and token creator information.
#[derive(Debug, Clone)]
pub struct TokenTrackingCache {
    /// Pool state cache
    pub pools: PoolStateCache,
    
    /// Token creator cache
    pub creators: TokenCreatorCache,
}

impl TokenTrackingCache {
    /// Create a new combined cache with the specified ETH threshold for pools
    pub fn new(eth_threshold: f64) -> Self {
        Self {
            pools: PoolStateCache::new(eth_threshold),
            creators: TokenCreatorCache::new(),
        }
    }
    
    /// Get comprehensive information about a token including pool and creator data
    pub async fn get_token_info(&self, token_address: &str) -> TokenInfo {
        let creator_info = self.creators.get_token_creator(token_address).await;
        let uses_private_mempool = self.creators.is_token_creator_private(token_address).await;
        
        // Find pools containing this token
        let pools = self.pools.get_all_pools().await;
        let related_pools: Vec<(String, PoolState)> = pools
            .into_iter()
            .filter(|(_, pool_state)| pool_state.token_address == token_address)
            .collect();
        
        TokenInfo {
            token_address: token_address.to_string(),
            creator_info,
            uses_private_mempool,
            related_pools,
        }
    }
}

/// Basic information about a token including pools and creator data.
#[derive(Debug, Clone)]
pub struct TokenInfo {
    /// Token contract address
    pub token_address: String,
    
    /// Creator information (if available)
    pub creator_info: Option<TokenCreatorState>,
    
    /// Whether the creator uses private mempool
    pub uses_private_mempool: bool,
    
    /// Pools that contain this token
    pub related_pools: Vec<(String, PoolState)>,
}

/// Unit tests for the PoolStateCache
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_update_and_get_pool() {
        let cache = PoolStateCache::new(0.05);
        
        // Create a test pool update
        let mut updates = HashMap::new();
        let pool_address = "0x1234567890abcdef1234567890abcdef12345678".to_string();
        let pool_update = PoolUpdate {
            eth_reserve: 5.0,
            token_reserve: 10000.0,
            token_address: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
            block_number: 12345,
            update_time: 1626000000.0,
        };
        updates.insert(pool_address.clone(), pool_update);
        
        // Update the cache
        let updated = cache.update_pools(updates.iter()).await;
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0], pool_address);
        
        // Get the pool from the cache
        let pool_state = cache.get_pool(&pool_address).await;
        assert!(pool_state.is_some());
        
        let pool_state = pool_state.unwrap();
        assert_eq!(pool_state.eth_reserve, 5.0);
        assert_eq!(pool_state.token_reserve, 10000.0);
        assert_eq!(pool_state.token_address, "0xabcdef1234567890abcdef1234567890abcdef12");
        assert_eq!(pool_state.last_updated_block, 12345);
    }
    
    #[tokio::test]
    async fn test_token_creator_cache() {
        let cache = TokenCreatorCache::new();
        
        // Create test creator data
        let mut creators = HashMap::new();
        let token_address = "0x1234567890abcdef1234567890abcdef12345678".to_string();
        let creator = crate::token_tracking::types::TokenCreator {
            creator_address: "0xabcdef1234567890abcdef1234567890abcdef12".to_string(),
            token_address: token_address.clone(),
            creation_block: 12345,
            creation_tx_hash: "0xdeadbeef".to_string(),
            creation_time: 1626000000.0,
            uses_private_mempool: false,
        };
        creators.insert(token_address.clone(), creator);
        
        // Update cache
        let updated_count = cache.update_creators(creators.iter()).await;
        assert_eq!(updated_count, 1);
        
        // Get creator information
        let creator_state = cache.get_token_creator(&token_address).await;
        assert!(creator_state.is_some());
        
        let creator_state = creator_state.unwrap();
        assert_eq!(creator_state.creator.creator_address, "0xabcdef1234567890abcdef1234567890abcdef12");
        assert!(!creator_state.creator.uses_private_mempool);
        
        // Test private mempool status
        assert!(!cache.is_token_creator_private(&token_address).await);
    }
    
    #[tokio::test]
    async fn test_combined_token_tracking_cache() {
        let cache = TokenTrackingCache::new(0.1);
        
        // Add a pool
        let mut pools = HashMap::new();
        let pool_address = "0x1111111111111111111111111111111111111111".to_string();
        let token_address = "0x2222222222222222222222222222222222222222".to_string();
        let pool_update = crate::token_tracking::types::PoolUpdate {
            eth_reserve: 10.0,
            token_reserve: 20000.0,
            token_address: token_address.clone(),
            block_number: 12345,
            update_time: 1626000000.0,
        };
        pools.insert(pool_address.clone(), pool_update);
        cache.pools.update_pools(pools.iter()).await;
        
        // Add creator for the same token
        let mut creators = HashMap::new();
        let creator = crate::token_tracking::types::TokenCreator {
            creator_address: "0x3333333333333333333333333333333333333333".to_string(),
            token_address: token_address.clone(),
            creation_block: 12340,
            creation_tx_hash: "0xfeedface".to_string(),
            creation_time: 1625999000.0,
            uses_private_mempool: true,
        };
        creators.insert(token_address.clone(), creator);
        cache.creators.update_creators(creators.iter()).await;
        
        // Get comprehensive token info
        let token_info = cache.get_token_info(&token_address).await;
        assert_eq!(token_info.token_address, token_address);
        assert!(token_info.creator_info.is_some());
        assert!(token_info.uses_private_mempool);
        assert_eq!(token_info.related_pools.len(), 1);
        assert_eq!(token_info.related_pools[0].0, pool_address);
    }
} 