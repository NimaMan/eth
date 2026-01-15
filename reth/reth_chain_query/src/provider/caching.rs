/// Caching layer - Efficient caching for frequently accessed data
///
/// This module provides caching for block timestamps, storage slots,
/// and other frequently accessed data to reduce database queries.
use alloy_primitives::Address;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Cache for block timestamps
pub struct BlockTimeCache {
    cache: Arc<RwLock<HashMap<u64, u64>>>,
    max_size: usize,
}

impl BlockTimeCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size: 10000, // Cache last 10k blocks
        }
    }

    pub fn get(&self, block_number: u64) -> Option<u64> {
        self.cache.read().get(&block_number).copied()
    }

    pub fn insert(&self, block_number: u64, timestamp: u64) {
        let mut cache = self.cache.write();

        // Simple eviction: remove oldest if at capacity
        if cache.len() >= self.max_size {
            // In production, use LRU cache
            if let Some(min_block) = cache.keys().min().copied() {
                cache.remove(&min_block);
            }
        }

        cache.insert(block_number, timestamp);
    }
}

/// Cache for known storage slot positions
pub struct SlotPositionCache {
    // Maps (contract, slot_name) -> position
    // e.g., (USDC, "balances") -> 2
    positions: Arc<RwLock<HashMap<(Address, String), u64>>>,
}

impl SlotPositionCache {
    pub fn new() -> Self {
        let mut positions = HashMap::new();

        // Pre-populate known slots for common tokens
        // USDC
        let usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
            .parse::<Address>()
            .unwrap();
        positions.insert((usdc, "balances".to_string()), 9); // USDC balances at slot 9

        // USDT
        let usdt = "0xdAC17F958D2ee523a2206206994597C13D831ec7"
            .parse::<Address>()
            .unwrap();
        positions.insert((usdt, "balances".to_string()), 2); // USDT balances at slot 2

        // WETH
        let weth = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
            .parse::<Address>()
            .unwrap();
        positions.insert((weth, "balanceOf".to_string()), 3); // WETH balances at slot 3

        // DAI
        let dai = "0x6B175474E89094C44Da98b954EedeAC495271d0F"
            .parse::<Address>()
            .unwrap();
        positions.insert((dai, "balances".to_string()), 2); // DAI balances at slot 2

        Self {
            positions: Arc::new(RwLock::new(positions)),
        }
    }

    pub fn get(&self, contract: Address, slot_name: &str) -> Option<u64> {
        self.positions
            .read()
            .get(&(contract, slot_name.to_string()))
            .copied()
    }

    pub fn insert(&self, contract: Address, slot_name: String, position: u64) {
        self.positions
            .write()
            .insert((contract, slot_name), position);
    }
}

/// Cache for token metadata
pub struct TokenMetadataCache {
    cache: Arc<RwLock<HashMap<Address, CachedTokenMetadata>>>,
}

#[derive(Clone)]
struct CachedTokenMetadata {
    name: String,
    symbol: String,
    decimals: u8,
}

impl TokenMetadataCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get(&self, token: Address) -> Option<(String, String, u8)> {
        self.cache
            .read()
            .get(&token)
            .map(|meta| (meta.name.clone(), meta.symbol.clone(), meta.decimals))
    }

    pub fn insert(&self, token: Address, name: String, symbol: String, decimals: u8) {
        self.cache.write().insert(
            token,
            CachedTokenMetadata {
                name,
                symbol,
                decimals,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_cache() {
        let cache = BlockTimeCache::new();

        // Test insertion and retrieval
        cache.insert(1000, 1234567890);
        assert_eq!(cache.get(1000), Some(1234567890));
        assert_eq!(cache.get(2000), None);
    }

    #[test]
    fn test_slot_position_cache() {
        let cache = SlotPositionCache::new();

        // Test pre-populated USDC
        let usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
            .parse::<Address>()
            .unwrap();
        assert_eq!(cache.get(usdc, "balances"), Some(9));

        // Test insertion
        let custom = Address::ZERO;
        cache.insert(custom, "myMapping".to_string(), 42);
        assert_eq!(cache.get(custom, "myMapping"), Some(42));
    }
}
