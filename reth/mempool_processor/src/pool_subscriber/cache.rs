// pool_subscriber/cache.rs
//
// Thread-safe cache for storing and retrieving the latest pool state information.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info};

use super::types::{PoolState, PoolUpdate};

/// Thread-safe cache for storing the latest pool ETH levels and metadata.
#[derive(Debug, Clone)]
pub struct PoolStateCache {
    /// Internal storage using RwLock for thread-safe concurrent access
    /// Maps pool addresses (as hex strings) to their current state
    storage: Arc<RwLock<HashMap<String, PoolState>>>,
    
    /// Threshold ETH value for scam detection
    eth_threshold: f64,
}

impl PoolStateCache {
    /// Create a new pool state cache with the specified ETH threshold
    pub fn new(eth_threshold: f64) -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            eth_threshold,
        }
    }
    
    /// Update the cache with a new PoolUpdatesMessage
    /// Returns a Vec of pool addresses that were updated
    pub fn update_pools<'a, I>(&self, pool_updates: I) -> Vec<String> 
    where
        I: IntoIterator<Item = (&'a String, &'a PoolUpdate)>,
    {
        let mut updated_addresses = Vec::new();
        
        // Lock for writing (exclusive access)
        let mut storage = match self.storage.write() {
            Ok(guard) => guard,
            Err(e) => {
                debug!("Failed to acquire write lock on pool state cache: {}", e);
                return Vec::new();
            }
        };
        
        // Process each pool update
        for (address, update) in pool_updates {
            let pool_state = PoolState::from(update.clone());
            storage.insert(address.clone(), pool_state);
            updated_addresses.push(address.clone());
        }
        
        // Log the update
        if !updated_addresses.is_empty() {
            debug!("Updated {} pools in state cache", updated_addresses.len());
            debug!("Updated pool addresses: {:?}", updated_addresses);
        }
        
        updated_addresses
    }
    
    /// Get the current state of a specific pool
    pub fn get_pool(&self, address: &str) -> Option<PoolState> {
        // Lock for reading (shared access)
        let storage = match self.storage.read() {
            Ok(guard) => guard,
            Err(e) => {
                debug!("Failed to acquire read lock on pool state cache: {}", e);
                return None;
            }
        };
        
        // Get and clone the pool state (if it exists)
        storage.get(address).cloned()
    }
    
    /// Get all pools in the cache
    pub fn get_all_pools(&self) -> HashMap<String, PoolState> {
        // Lock for reading (shared access)
        let storage = match self.storage.read() {
            Ok(guard) => guard,
            Err(e) => {
                debug!("Failed to acquire read lock on pool state cache: {}", e);
                return HashMap::new();
            }
        };
        
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
    pub fn get_pool_count(&self) -> usize {
        // Lock for reading (shared access)
        let storage = match self.storage.read() {
            Ok(guard) => guard,
            Err(e) => {
                debug!("Failed to acquire read lock on pool state cache: {}", e);
                return 0;
            }
        };
        
        storage.len()
    }
}

/// Unit tests for the PoolStateCache
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_update_and_get_pool() {
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
        let updated = cache.update_pools(updates.iter());
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0], pool_address);
        
        // Get the pool from the cache
        let pool_state = cache.get_pool(&pool_address);
        assert!(pool_state.is_some());
        
        let pool_state = pool_state.unwrap();
        assert_eq!(pool_state.eth_reserve, 5.0);
        assert_eq!(pool_state.token_reserve, 10000.0);
        assert_eq!(pool_state.token_address, "0xabcdef1234567890abcdef1234567890abcdef12");
        assert_eq!(pool_state.last_updated_block, 12345);
    }
} 