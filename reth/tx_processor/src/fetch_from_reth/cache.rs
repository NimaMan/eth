//! Caching implementation for the fetch_from_reth module
//!
//! This module provides LRU caching with TTL support for transaction data
//! to improve performance of repeated queries.

use crate::fetch_from_reth::{config::CacheConfig, provider::TransactionData};
use alloy_primitives::B256;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Cache entry with TTL support
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached transaction data
    data: TransactionData,
    /// When this entry was created
    created_at: Instant,
    /// When this entry was last accessed
    last_accessed: Instant,
}

impl CacheEntry {
    fn new(data: TransactionData) -> Self {
        let now = Instant::now();
        Self {
            data,
            created_at: now,
            last_accessed: now,
        }
    }
    
    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
    
    fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }
}

/// LRU cache with TTL support for transaction data
#[derive(Debug)]
pub struct TransactionCache {
    /// Configuration for the cache
    config: CacheConfig,
    /// Storage for cache entries
    entries: Arc<RwLock<HashMap<B256, CacheEntry>>>,
    /// LRU tracking (transaction hash -> access time)
    access_order: Arc<RwLock<Vec<B256>>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache performance statistics
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// Total number of cache lookups
    pub total_requests: u64,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of expired entries removed
    pub expired_removals: u64,
    /// Number of LRU evictions
    pub lru_evictions: u64,
    /// Number of entries currently in cache
    pub current_size: usize,
    /// Maximum size reached
    pub max_size_reached: usize,
}

impl CacheStats {
    /// Calculate cache hit rate as a percentage
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.hits as f64 / self.total_requests as f64) * 100.0
        }
    }
    
    /// Calculate cache miss rate as a percentage
    pub fn miss_rate(&self) -> f64 {
        100.0 - self.hit_rate()
    }
    
    /// Check if cache is performing well (>80% hit rate)
    pub fn is_performing_well(&self) -> bool {
        self.hit_rate() > 80.0 && self.total_requests > 100
    }
}

impl TransactionCache {
    /// Create a new transaction cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::with_capacity(config.initial_capacity))),
            access_order: Arc::new(RwLock::new(Vec::with_capacity(config.initial_capacity))),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            config,
        }
    }
    
    /// Get a transaction from the cache
    pub fn get(&self, tx_hash: &B256) -> Option<TransactionData> {
        if !self.config.is_enabled() {
            return None;
        }
        
        // Update stats
        if self.config.enable_stats {
            if let Ok(mut stats) = self.stats.write() {
                stats.total_requests += 1;
            }
        }
        
        // Try to get entry
        let mut entries = self.entries.write().ok()?;
        let mut access_order = self.access_order.write().ok()?;
        
        if let Some(entry) = entries.get_mut(tx_hash) {
            // Check if expired
            if entry.is_expired(self.config.ttl) {
                entries.remove(tx_hash);
                access_order.retain(|h| h != tx_hash);
                
                if self.config.enable_stats {
                    if let Ok(mut stats) = self.stats.write() {
                        stats.misses += 1;
                        stats.expired_removals += 1;
                        stats.current_size = entries.len();
                    }
                }
                return None;
            }
            
            // Update access time and order
            entry.touch();
            
            // Move to end of access order (most recently used)
            if let Some(pos) = access_order.iter().position(|h| h == tx_hash) {
                access_order.remove(pos);
            }
            access_order.push(*tx_hash);
            
            // Update stats
            if self.config.enable_stats {
                if let Ok(mut stats) = self.stats.write() {
                    stats.hits += 1;
                }
            }
            
            Some(entry.data.clone())
        } else {
            // Cache miss
            if self.config.enable_stats {
                if let Ok(mut stats) = self.stats.write() {
                    stats.misses += 1;
                }
            }
            None
        }
    }
    
    /// Put a transaction into the cache
    pub fn put(&self, tx_hash: B256, data: TransactionData) {
        if !self.config.is_enabled() {
            return;
        }
        
        let mut entries = match self.entries.write() {
            Ok(entries) => entries,
            Err(_) => return,
        };
        
        let mut access_order = match self.access_order.write() {
            Ok(access_order) => access_order,
            Err(_) => return,
        };
        
        // If already exists, update and move to end
        if entries.contains_key(&tx_hash) {
            entries.insert(tx_hash, CacheEntry::new(data));
            
            // Move to end of access order
            if let Some(pos) = access_order.iter().position(|h| h == &tx_hash) {
                access_order.remove(pos);
            }
            access_order.push(tx_hash);
            return;
        }
        
        // Check if we need to evict
        while entries.len() >= self.config.max_size {
            if let Some(oldest_hash) = access_order.first().cloned() {
                entries.remove(&oldest_hash);
                access_order.remove(0);
                
                if self.config.enable_stats {
                    if let Ok(mut stats) = self.stats.write() {
                        stats.lru_evictions += 1;
                    }
                }
            } else {
                break;
            }
        }
        
        // Insert new entry
        entries.insert(tx_hash, CacheEntry::new(data));
        access_order.push(tx_hash);
        
        // Update stats
        if self.config.enable_stats {
            if let Ok(mut stats) = self.stats.write() {
                stats.current_size = entries.len();
                if stats.current_size > stats.max_size_reached {
                    stats.max_size_reached = stats.current_size;
                }
            }
        }
    }
    
    /// Remove expired entries from the cache
    pub fn cleanup_expired(&self) {
        if !self.config.is_enabled() {
            return;
        }
        
        let mut entries = match self.entries.write() {
            Ok(entries) => entries,
            Err(_) => return,
        };
        
        let mut access_order = match self.access_order.write() {
            Ok(access_order) => access_order,
            Err(_) => return,
        };
        
        let mut expired_count = 0;
        let expired_hashes: Vec<B256> = entries
            .iter()
            .filter_map(|(hash, entry)| {
                if entry.is_expired(self.config.ttl) {
                    Some(*hash)
                } else {
                    None
                }
            })
            .collect();
        
        for hash in expired_hashes {
            entries.remove(&hash);
            access_order.retain(|h| h != &hash);
            expired_count += 1;
        }
        
        // Update stats
        if self.config.enable_stats && expired_count > 0 {
            if let Ok(mut stats) = self.stats.write() {
                stats.expired_removals += expired_count;
                stats.current_size = entries.len();
            }
        }
    }
    
    /// Clear all entries from the cache
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
        if let Ok(mut access_order) = self.access_order.write() {
            access_order.clear();
        }
        if let Ok(mut stats) = self.stats.write() {
            stats.current_size = 0;
        }
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        if let Ok(stats) = self.stats.read() {
            stats.clone()
        } else {
            CacheStats::default()
        }
    }
    
    /// Get current cache size
    pub fn size(&self) -> usize {
        if let Ok(entries) = self.entries.read() {
            entries.len()
        } else {
            0
        }
    }
    
    /// Check if cache contains a specific transaction
    pub fn contains(&self, tx_hash: &B256) -> bool {
        if let Ok(entries) = self.entries.read() {
            entries.contains_key(tx_hash)
        } else {
            false
        }
    }
    
    /// Get cache capacity utilization as a percentage
    pub fn utilization(&self) -> f64 {
        if self.config.max_size == 0 {
            return 0.0;
        }
        (self.size() as f64 / self.config.max_size as f64) * 100.0
    }
}

impl Clone for TransactionCache {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            entries: Arc::clone(&self.entries),
            access_order: Arc::clone(&self.access_order),
            stats: Arc::clone(&self.stats),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, U256};
    use std::str::FromStr;
    use std::thread;
    
    fn create_test_transaction(hash_suffix: u8) -> (B256, TransactionData) {
        let mut hash_bytes = [0u8; 32];
        hash_bytes[31] = hash_suffix;
        let hash = B256::from(hash_bytes);
        
        let data = TransactionData {
            hash,
            from: Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
            to: Some(Address::from_str("0x0000000000000000000000000000000000000002").unwrap()),
            value: U256::from(1000),
            gas_limit: 21000,
            gas_used: 21000,
            gas_price: U256::from(20000000000u64),
            nonce: 1,
            block_number: 1000,
            block_hash: B256::ZERO,
            transaction_index: 0,
            input: vec![].into(),
            receipt_status: true,
            contract_address: None,
            logs: vec![],
        };
        
        (hash, data)
    }
    
    #[test]
    fn test_cache_basic_operations() {
        let config = CacheConfig::default();
        let cache = TransactionCache::new(config);
        
        let (hash, data) = create_test_transaction(1);
        
        // Should be empty initially
        assert!(cache.get(&hash).is_none());
        assert_eq!(cache.size(), 0);
        
        // Put and get
        cache.put(hash, data.clone());
        assert_eq!(cache.size(), 1);
        assert!(cache.contains(&hash));
        
        let retrieved = cache.get(&hash).unwrap();
        assert_eq!(retrieved.hash, hash);
        assert_eq!(retrieved.value, data.value);
    }
    
    #[test]
    fn test_cache_lru_eviction() {
        let config = CacheConfig::default().with_max_size(2);
        let cache = TransactionCache::new(config);
        
        let (hash1, data1) = create_test_transaction(1);
        let (hash2, data2) = create_test_transaction(2);
        let (hash3, data3) = create_test_transaction(3);
        
        // Fill cache to capacity
        cache.put(hash1, data1);
        cache.put(hash2, data2);
        assert_eq!(cache.size(), 2);
        
        // Adding third should evict first (LRU)
        cache.put(hash3, data3);
        assert_eq!(cache.size(), 2);
        assert!(!cache.contains(&hash1)); // Should be evicted
        assert!(cache.contains(&hash2));
        assert!(cache.contains(&hash3));
    }
    
    #[test]
    fn test_cache_access_order() {
        let config = CacheConfig::default().with_max_size(2);
        let cache = TransactionCache::new(config);
        
        let (hash1, data1) = create_test_transaction(1);
        let (hash2, data2) = create_test_transaction(2);
        let (hash3, data3) = create_test_transaction(3);
        
        // Fill cache
        cache.put(hash1, data1);
        cache.put(hash2, data2);
        
        // Access hash1 to make it most recent
        cache.get(&hash1);
        
        // Adding third should evict hash2 (least recently used)
        cache.put(hash3, data3);
        assert!(cache.contains(&hash1)); // Should remain (recently accessed)
        assert!(!cache.contains(&hash2)); // Should be evicted
        assert!(cache.contains(&hash3));
    }
    
    #[test]
    fn test_cache_ttl_expiration() {
        let config = CacheConfig::default()
            .with_ttl(Duration::from_millis(10))
            .with_max_size(10);
        let cache = TransactionCache::new(config);
        
        let (hash, data) = create_test_transaction(1);
        
        // Put data
        cache.put(hash, data);
        assert!(cache.contains(&hash));
        
        // Should still be there immediately
        assert!(cache.get(&hash).is_some());
        
        // Wait for expiration
        thread::sleep(Duration::from_millis(15));
        
        // Should be expired now
        assert!(cache.get(&hash).is_none());
    }
    
    #[test]
    fn test_cache_statistics() {
        let config = CacheConfig::default().with_stats(true);
        let cache = TransactionCache::new(config);
        
        let (hash1, data1) = create_test_transaction(1);
        let (hash2, _) = create_test_transaction(2);
        
        // Put data
        cache.put(hash1, data1);
        
        // Get hit
        cache.get(&hash1);
        
        // Get miss
        cache.get(&hash2);
        
        let stats = cache.stats();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate(), 50.0);
    }
    
    #[test]
    fn test_cache_disabled() {
        let config = CacheConfig::disabled();
        let cache = TransactionCache::new(config);
        
        let (hash, data) = create_test_transaction(1);
        
        // Should not cache anything
        cache.put(hash, data);
        assert_eq!(cache.size(), 0);
        assert!(cache.get(&hash).is_none());
    }
    
    #[test]
    fn test_cache_cleanup_expired() {
        let config = CacheConfig::default()
            .with_ttl(Duration::from_millis(10))
            .with_max_size(10);
        let cache = TransactionCache::new(config);
        
        let (hash1, data1) = create_test_transaction(1);
        let (hash2, data2) = create_test_transaction(2);
        
        // Put data
        cache.put(hash1, data1);
        thread::sleep(Duration::from_millis(15)); // Let first expire
        cache.put(hash2, data2); // This one won't expire yet
        
        assert_eq!(cache.size(), 2); // Both still in cache
        
        // Clean up expired
        cache.cleanup_expired();
        
        assert_eq!(cache.size(), 1); // Only non-expired remains
        assert!(!cache.contains(&hash1));
        assert!(cache.contains(&hash2));
    }
    
    #[test]
    fn test_cache_clear() {
        let config = CacheConfig::default();
        let cache = TransactionCache::new(config);
        
        let (hash, data) = create_test_transaction(1);
        cache.put(hash, data);
        assert_eq!(cache.size(), 1);
        
        cache.clear();
        assert_eq!(cache.size(), 0);
        assert!(!cache.contains(&hash));
    }
    
    #[test]
    fn test_cache_utilization() {
        let config = CacheConfig::default().with_max_size(10);
        let cache = TransactionCache::new(config);
        
        assert_eq!(cache.utilization(), 0.0);
        
        // Fill half the cache
        for i in 0..5 {
            let (hash, data) = create_test_transaction(i);
            cache.put(hash, data);
        }
        
        assert_eq!(cache.utilization(), 50.0);
    }
}