use chrono::{DateTime, Utc};
/// Timestamp Cache
///
/// LRU cache for block-timestamp mappings to improve performance.
use std::collections::HashMap;
use std::sync::RwLock;

/// Thread-safe cache for block timestamps
pub struct TimestampCache {
    cache: RwLock<LruCache>,
}

impl TimestampCache {
    /// Create new cache with specified max size
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: RwLock::new(LruCache::new(max_size)),
        }
    }

    /// Get timestamp for a block if cached
    pub fn get_timestamp(&self, block_number: u64) -> Option<DateTime<Utc>> {
        self.cache.read().unwrap().get(&block_number).copied()
    }

    /// Insert a block-timestamp mapping
    pub fn insert(&self, block_number: u64, timestamp: DateTime<Utc>) {
        self.cache.write().unwrap().insert(block_number, timestamp);
    }

    /// Find closest cached block for a timestamp
    pub fn find_block_for_timestamp(&self, target: DateTime<Utc>) -> Option<u64> {
        let cache = self.cache.read().unwrap();

        // Find the closest cached block
        let mut closest_block = None;
        let mut min_diff = i64::MAX;

        for (&block, &timestamp) in cache.iter() {
            let diff = (timestamp.timestamp() - target.timestamp()).abs();
            if diff < min_diff {
                min_diff = diff;
                closest_block = Some(block);
            }
        }

        // Only return if within 1 hour (300 blocks)
        if min_diff < 3600 {
            closest_block
        } else {
            None
        }
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.cache.write().unwrap().clear();
    }

    /// Get cache size
    pub fn size(&self) -> usize {
        self.cache.read().unwrap().len()
    }
}

/// Simple LRU cache implementation
struct LruCache {
    map: HashMap<u64, DateTime<Utc>>,
    order: Vec<u64>,
    max_size: usize,
}

impl LruCache {
    fn new(max_size: usize) -> Self {
        Self {
            map: HashMap::with_capacity(max_size),
            order: Vec::with_capacity(max_size),
            max_size,
        }
    }

    fn get(&self, key: &u64) -> Option<&DateTime<Utc>> {
        self.map.get(key)
    }

    fn insert(&mut self, key: u64, value: DateTime<Utc>) {
        // Remove if already exists
        if let Some(pos) = self.order.iter().position(|&k| k == key) {
            self.order.remove(pos);
        }

        // Add to end
        self.order.push(key);
        self.map.insert(key, value);

        // Evict oldest if over capacity
        while self.order.len() > self.max_size {
            if let Some(oldest) = self.order.first() {
                let oldest = *oldest;
                self.order.remove(0);
                self.map.remove(&oldest);
            }
        }
    }

    fn iter(&self) -> impl Iterator<Item = (&u64, &DateTime<Utc>)> {
        self.map.iter()
    }

    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let cache = TimestampCache::new(3);
        let now = Utc::now();

        // Insert some values
        cache.insert(100, now);
        cache.insert(200, now + chrono::Duration::seconds(1200));
        cache.insert(300, now + chrono::Duration::seconds(2400));

        // Check retrieval
        assert_eq!(cache.get_timestamp(100), Some(now));
        assert_eq!(
            cache.get_timestamp(200),
            Some(now + chrono::Duration::seconds(1200))
        );
        assert_eq!(cache.get_timestamp(400), None);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = TimestampCache::new(2);
        let now = Utc::now();

        cache.insert(100, now);
        cache.insert(200, now + chrono::Duration::seconds(1200));
        cache.insert(300, now + chrono::Duration::seconds(2400)); // Should evict 100

        assert_eq!(cache.get_timestamp(100), None); // Evicted
        assert_eq!(
            cache.get_timestamp(200),
            Some(now + chrono::Duration::seconds(1200))
        );
        assert_eq!(
            cache.get_timestamp(300),
            Some(now + chrono::Duration::seconds(2400))
        );
    }
}
