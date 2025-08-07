// token_tracking/cache.rs
//
// Thread-safe cache for storing and retrieving pool state and token creator information.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::types::{PoolState, PoolUpdate, TokenCreatorState, SimulationData};

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
    
    /// Check if an address is a known pool address
    pub async fn is_pool_address(&self, address: &str) -> bool {
        let storage = self.storage.read().await;
        storage.contains_key(address)
    }
}


/// Combined cache for both pool states and token creator information.
#[derive(Debug, Clone)]
pub struct TokenTrackingCache {
    /// Pool state cache
    pub pools: PoolStateCache,
    
    /// Full token information storage (includes simulation data)
    /// Maps token addresses to complete token information
    tokens: Arc<RwLock<HashMap<String, super::types::TokenInfo>>>,
    
    /// Pre-computed sets for fast lookups
    all_creators: Arc<RwLock<std::collections::HashSet<String>>>,
    all_pools: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl TokenTrackingCache {
    /// Create a new combined cache with the specified ETH threshold for pools
    pub fn new(eth_threshold: f64) -> Self {
        Self {
            pools: PoolStateCache::new(eth_threshold),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            all_creators: Arc::new(RwLock::new(std::collections::HashSet::new())),
            all_pools: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }
    
    /// Get comprehensive information about a token including pool and creator data
    pub async fn get_token_info(&self, token_address: &str) -> TokenInfo {
        // Try to get from full token info first
        if let Some(token) = self.get_token(token_address).await {
            return TokenInfo {
                token_address: token.token_address,
                creator_info: None, // TokenCreatorState has been removed
                uses_private_mempool: false, // This info is not in TokenInfo type
                related_pools: self.get_pools_for_token(token_address).await,
            };
        }
        
        // If not found, return minimal info
        TokenInfo {
            token_address: token_address.to_string(),
            creator_info: None,
            uses_private_mempool: false,
            related_pools: vec![],
        }
    }
    
    /// Update token tax information
    pub async fn update_token_tax(&self, token_address: &str, buy_tax: Option<u8>, sell_tax: Option<u8>) {
        let mut tokens_guard = self.tokens.write().await;
        
        if let Some(token_info) = tokens_guard.get_mut(token_address) {
            // Update tax values
            if buy_tax.is_some() {
                token_info.buy_tax = buy_tax;
            }
            if sell_tax.is_some() {
                token_info.sell_tax = sell_tax;
            }
            
            info!("Updated tax for token {}: Buy: {:?}%, Sell: {:?}%", 
                  token_address, buy_tax, sell_tax);
        } else {
            warn!("Attempted to update tax for unknown token: {}", token_address);
        }
    }
    
    
    /// Get all tokens created by a specific address
    pub async fn get_tokens_by_creator(&self, creator_address: &str) -> Vec<String> {
        let tokens_guard = self.tokens.read().await;
        tokens_guard.iter()
            .filter(|(_, token_info)| token_info.creator_address == creator_address)
            .map(|(token_addr, _)| token_addr.clone())
            .collect()
    }
    
    /// Get pool information for a specific pool address
    pub async fn get_pool_by_address(&self, pool_address: &str) -> Option<PoolState> {
        self.pools.get_pool(pool_address).await
    }
    
    /// Get all pools for a specific token
    pub async fn get_pools_for_token(&self, token_address: &str) -> Vec<(String, PoolState)> {
        let all_pools = self.pools.get_all_pools().await;
        all_pools.into_iter()
            .filter(|(_, pool)| pool.token_address.eq_ignore_ascii_case(token_address))
            .collect()
    }
    
    /// Get all unique token addresses tracked in the system
    /// This includes tokens from both pools and creator information
    pub async fn get_all_token_addresses(&self) -> Vec<String> {
        use std::collections::HashSet;
        
        // Get all tokens from pools
        let pools = self.pools.get_all_pools().await;
        let mut token_set: HashSet<String> = pools.values()
            .map(|pool| pool.token_address.clone())
            .collect();
        
        // Add all tokens from the tokens HashMap
        let tokens_guard = self.tokens.read().await;
        for token_addr in tokens_guard.keys() {
            token_set.insert(token_addr.clone());
        }
        
        // Convert to Vec and return
        token_set.into_iter().collect()
    }
    
    /// Check if an address is associated with any token (as creator or pool)
    pub async fn is_tracked_address(&self, address: &str) -> bool {
        // Check if it's a creator
        if self.is_creator(address).await {
            return true;
        }
        
        // Check if it's a pool
        if self.pools.get_pool(address).await.is_some() {
            return true;
        }
        
        false
    }
    
    /// Update full token information from Python
    pub async fn update_token(&self, mut token_info: super::types::TokenInfo) {
        let token_address = token_info.token_address.clone();
        
        // Convert Python tax values (0-100 float) to u8
        token_info.buy_tax = token_info.buy_tax_python.map(|t| t.round() as u8);
        token_info.sell_tax = token_info.sell_tax_python.map(|t| t.round() as u8);
        
        // Update the pre-computed sets
        let mut all_creators_guard = self.all_creators.write().await;
        all_creators_guard.insert(token_info.creator_address.clone());
        // Add all tax setter addresses
        for tax_setter in &token_info.tax_setter_addresses {
            all_creators_guard.insert(tax_setter.clone());
        }
        all_creators_guard.insert(token_info.current_owner.clone());
        drop(all_creators_guard);
        
        let mut all_pools_guard = self.all_pools.write().await;
        for pool_address in token_info.pools.keys() {
            all_pools_guard.insert(pool_address.clone());
        }
        drop(all_pools_guard);
        
        // Store the full token information
        let mut tokens_guard = self.tokens.write().await;
        tokens_guard.insert(token_address, token_info);
    }
    
    /// Get full token information (with simulation data if available)
    pub async fn get_token(&self, token_address: &str) -> Option<super::types::TokenInfo> {
        let tokens_guard = self.tokens.read().await;
        tokens_guard.get(token_address).cloned()
    }
    
    /// Get the token created by a specific creator address
    /// Returns the first token if creator has multiple tokens
    pub async fn get_token_for_creator(&self, creator_address: &str) -> Option<super::types::TokenInfo> {
        let tokens_guard = self.tokens.read().await;
        tokens_guard.values()
            .find(|token| token.creator_address.eq_ignore_ascii_case(creator_address))
            .cloned()
    }
    
    /// Set simulation results for a token
    pub async fn set_simulation_results(&self, token_address: &str, simulation_data: SimulationData) {
        let mut tokens_guard = self.tokens.write().await;
        if let Some(token_info) = tokens_guard.get_mut(token_address) {
            token_info.simulation_data = Some(simulation_data);
            info!("Updated simulation results for token {}", token_address);
        } else {
            warn!("Attempted to set simulation results for unknown token: {}", token_address);
        }
    }
    
    /// Get all creator addresses for fast mempool filtering
    pub async fn get_all_creators(&self) -> std::collections::HashSet<String> {
        let guard = self.all_creators.read().await;
        guard.clone()
    }
    
    /// Get all pool addresses for fast mempool filtering
    pub async fn get_all_pools(&self) -> std::collections::HashSet<String> {
        let guard = self.all_pools.read().await;
        guard.clone()
    }
    
    /// Check if an address is a creator/owner/tax setter
    pub async fn is_creator(&self, address: &str) -> bool {
        let guard = self.all_creators.read().await;
        guard.contains(address)
    }
    
    /// Get the primary pool for a token (highest liquidity)
    pub async fn get_primary_pool(&self, token_address: &str) -> Option<super::types::PoolInfo> {
        let tokens_guard = self.tokens.read().await;
        if let Some(token_info) = tokens_guard.get(token_address) {
            // Find pool with highest ETH reserve
            token_info.pools.values()
                .max_by(|a, b| a.denom_reserve.partial_cmp(&b.denom_reserve).unwrap())
                .cloned()
        } else {
            None
        }
    }
    
    /// Update creators from TokenCreator data (compatibility method)
    /// Returns the number of creators updated
    pub async fn update_creators<'a, I>(&self, creator_updates: I) -> usize 
    where
        I: IntoIterator<Item = (&'a String, &'a super::types::TokenCreator)>,
    {
        let mut updated_count = 0;
        
        for (token_address, creator) in creator_updates {
            // Check if we already have this token
            let existing_token = self.get_token(token_address).await;
            
            if let Some(mut token_info) = existing_token {
                // Update existing token info
                token_info.creator_address = creator.creator_address.clone();
                token_info.creation_block = creator.creation_block;
                token_info.creation_txn = creator.creation_tx_hash.clone();
                self.update_token(token_info).await;
            } else {
                // Create new token info from creator data
                let token_info = super::types::TokenInfo {
                    token_address: token_address.clone(),
                    creator_address: creator.creator_address.clone(),
                    creation_block: creator.creation_block,
                    creation_txn: creator.creation_tx_hash.clone(),
                    trading_enabled: false,
                    trading_enabled_txn: None,
                    current_owner: creator.creator_address.clone(),
                    ownership_renounced: false,
                    is_scam: false,
                    scam_label: None,
                    latest_activity_block: creator.creation_block,
                    buy_tax: None,
                    sell_tax: None,
                    last_tax_update_txn: None,
                    pools: HashMap::new(),
                    symbol: None,
                    name: None,
                    decimals: None,
                    total_supply: None,
                    tax_setter_addresses: vec![],
                    buy_tax_setter: None,
                    sell_tax_setter: None,
                    buy_tax_python: None,
                    sell_tax_python: None,
                    simulation_data: None,
                };
                self.update_token(token_info).await;
            }
            
            updated_count += 1;
        }
        
        updated_count
    }
    
    /// Get the count of creators (for compatibility)
    pub async fn get_creator_count(&self) -> usize {
        let guard = self.all_creators.read().await;
        guard.len()
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
            pool_type: "V2".to_string(),
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
    
    // TokenCreatorCache test removed - functionality merged into TokenTrackingCache
    
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
            pool_type: "V2".to_string(),
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
        cache.update_creators(creators.iter()).await;
        
        // Get comprehensive token info
        let token_info = cache.get_token_info(&token_address).await;
        assert_eq!(token_info.token_address, token_address);
        assert!(token_info.creator_info.is_some());
        assert!(token_info.uses_private_mempool);
        assert_eq!(token_info.related_pools.len(), 1);
        assert_eq!(token_info.related_pools[0].0, pool_address);
    }
    
    #[tokio::test]
    async fn test_get_all_token_addresses() {
        let cache = TokenTrackingCache::new(0.1);
        
        // Add multiple tokens through pools
        let mut pools = HashMap::new();
        let token1 = "0x1111111111111111111111111111111111111111".to_string();
        let token2 = "0x2222222222222222222222222222222222222222".to_string();
        
        pools.insert("0xaaa".to_string(), crate::token_tracking::types::PoolUpdate {
            eth_reserve: 5.0,
            token_reserve: 10000.0,
            token_address: token1.clone(),
            pool_type: "V2".to_string(),
            block_number: 12345,
            update_time: 1626000000.0,
        });
        
        pools.insert("0xbbb".to_string(), crate::token_tracking::types::PoolUpdate {
            eth_reserve: 10.0,
            token_reserve: 20000.0,
            token_address: token2.clone(),
            pool_type: "V3".to_string(),
            block_number: 12346,
            update_time: 1626000001.0,
        });
        
        cache.pools.update_pools(pools.iter()).await;
        
        // Add a token through creator (different from pools)
        let token3 = "0x3333333333333333333333333333333333333333".to_string();
        let mut creators = HashMap::new();
        creators.insert(token3.clone(), crate::token_tracking::types::TokenCreator {
            creator_address: "0xcreator".to_string(),
            token_address: token3.clone(),
            creation_block: 12340,
            creation_tx_hash: "0xhash".to_string(),
            creation_time: 1625999000.0,
            uses_private_mempool: false,
        });
        cache.update_creators(creators.iter()).await;
        
        // Get all token addresses
        let all_tokens = cache.get_all_token_addresses().await;
        assert_eq!(all_tokens.len(), 3);
        assert!(all_tokens.contains(&token1));
        assert!(all_tokens.contains(&token2));
        assert!(all_tokens.contains(&token3));
    }
} 