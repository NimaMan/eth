/*
 * Liquidity Pools Tracker Module
 * 
 * This module tracks Ethereum token liquidity pools:
 * - Monitor pool balances in real-time
 * - Track significant changes in pool liquidity
 * - Detect potential rug pulls or market manipulation
 * - Support for Uniswap V2, V3, and V4 pools
 */

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use ethers::types::{H160, U256};
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};

/// Represents a liquidity pool state at a specific block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    pub pool_address: H160,
    pub token0_address: H160,
    pub token1_address: H160,
    pub reserve0: U256,
    pub reserve1: U256,
    pub total_supply: U256,
    pub block_number: u64,
    pub timestamp: u64,
    pub pool_type: PoolType,
}

/// Types of pools we support
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PoolType {
    UniswapV2,
    UniswapV3,
    UniswapV4,
    Unknown,
}

/// Pool update event
#[derive(Debug, Clone)]
pub struct PoolUpdate {
    pub pool_address: H160,
    pub reserve0_delta: i128,
    pub reserve1_delta: i128,
    pub block_number: u64,
    pub tx_hash: H160,
}

/// Tracks liquidity pools and their state changes
pub struct PoolTracker {
    /// Current state of all tracked pools
    pools: Arc<RwLock<HashMap<H160, PoolState>>>,
    /// Pool creation events we're watching for
    watched_factories: Vec<H160>,
    /// Minimum liquidity threshold to track a pool (in ETH)
    min_liquidity_threshold: U256,
}

impl PoolTracker {
    /// Create a new pool tracker
    pub fn new(min_liquidity_eth: f64) -> Self {
        let min_liquidity_threshold = U256::from((min_liquidity_eth * 1e18) as u128);
        
        // Well-known factory addresses
        let watched_factories = vec![
            // Uniswap V2 Factory
            "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".parse::<H160>().unwrap(),
            // Uniswap V3 Factory
            "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse::<H160>().unwrap(),
            // Sushiswap Factory
            "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac".parse::<H160>().unwrap(),
        ];
        
        info!("Initialized pool tracker with {} ETH minimum liquidity threshold", min_liquidity_eth);
        
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            watched_factories,
            min_liquidity_threshold,
        }
    }
    
    /// Register a new pool
    pub fn register_pool(&self, pool_state: PoolState) -> bool {
        // Check if pool meets minimum liquidity threshold
        let eth_value = self.estimate_pool_eth_value(&pool_state);
        if eth_value < self.min_liquidity_threshold {
            debug!("Pool {} below minimum liquidity threshold, not tracking", pool_state.pool_address);
            return false;
        }
        
        let mut pools = self.pools.write().unwrap();
        let pool_address = pool_state.pool_address;
        
        if pools.contains_key(&pool_address) {
            debug!("Pool {} already registered", pool_address);
            return false;
        }
        
        info!("Registering new {} pool: {} with reserves ({}, {})", 
              pool_state.pool_type.name(), 
              pool_address,
              pool_state.reserve0,
              pool_state.reserve1
        );
        
        pools.insert(pool_address, pool_state);
        true
    }
    
    /// Update pool reserves
    pub fn update_pool(&self, update: PoolUpdate) -> Option<PoolState> {
        let mut pools = self.pools.write().unwrap();
        
        if let Some(pool) = pools.get_mut(&update.pool_address) {
            // Apply deltas
            if update.reserve0_delta >= 0 {
                pool.reserve0 += U256::from(update.reserve0_delta as u128);
            } else {
                let delta = U256::from((-update.reserve0_delta) as u128);
                pool.reserve0 = pool.reserve0.saturating_sub(delta);
            }
            
            if update.reserve1_delta >= 0 {
                pool.reserve1 += U256::from(update.reserve1_delta as u128);
            } else {
                let delta = U256::from((-update.reserve1_delta) as u128);
                pool.reserve1 = pool.reserve1.saturating_sub(delta);
            }
            
            pool.block_number = update.block_number;
            
            debug!("Updated pool {} reserves to ({}, {})", 
                   update.pool_address, pool.reserve0, pool.reserve1);
            
            Some(pool.clone())
        } else {
            warn!("Attempted to update unregistered pool: {}", update.pool_address);
            None
        }
    }
    
    /// Get current pool state
    pub fn get_pool(&self, pool_address: &H160) -> Option<PoolState> {
        let pools = self.pools.read().unwrap();
        pools.get(pool_address).cloned()
    }
    
    /// Get all tracked pools
    pub fn get_all_pools(&self) -> Vec<PoolState> {
        let pools = self.pools.read().unwrap();
        pools.values().cloned().collect()
    }
    
    /// Check if an address is a watched factory
    pub fn is_watched_factory(&self, address: &H160) -> bool {
        self.watched_factories.contains(address)
    }
    
    /// Detect potential rug pull based on liquidity changes
    pub fn detect_rug_pull(&self, pool_address: &H160, new_reserves: (U256, U256)) -> bool {
        let pools = self.pools.read().unwrap();
        
        if let Some(pool) = pools.get(pool_address) {
            // Check if either reserve dropped by more than 90%
            let reserve0_drop = if pool.reserve0 > U256::zero() {
                let drop = pool.reserve0.saturating_sub(new_reserves.0);
                drop.as_u128() as f64 / pool.reserve0.as_u128() as f64
            } else {
                0.0
            };
            
            let reserve1_drop = if pool.reserve1 > U256::zero() {
                let drop = pool.reserve1.saturating_sub(new_reserves.1);
                drop.as_u128() as f64 / pool.reserve1.as_u128() as f64
            } else {
                0.0
            };
            
            if reserve0_drop > 0.9 || reserve1_drop > 0.9 {
                warn!("POTENTIAL RUG PULL DETECTED in pool {}: reserve drops: {:.1}%, {:.1}%", 
                      pool_address, reserve0_drop * 100.0, reserve1_drop * 100.0);
                return true;
            }
        }
        
        false
    }
    
    /// Estimate pool value in ETH (simplified)
    fn estimate_pool_eth_value(&self, pool: &PoolState) -> U256 {
        // Simplified: assume one token is WETH or worth ~1 ETH per 1e18 units
        // In production, you'd use price oracles
        std::cmp::max(pool.reserve0, pool.reserve1)
    }
    
    /// Get pool statistics
    pub fn get_stats(&self) -> PoolTrackerStats {
        let pools = self.pools.read().unwrap();
        
        let mut stats = PoolTrackerStats {
            total_pools: pools.len(),
            pools_by_type: HashMap::new(),
            total_value_locked: U256::zero(),
        };
        
        for pool in pools.values() {
            *stats.pools_by_type.entry(pool.pool_type.clone()).or_insert(0) += 1;
            stats.total_value_locked += self.estimate_pool_eth_value(pool);
        }
        
        stats
    }
}

/// Pool tracker statistics
#[derive(Debug, Clone)]
pub struct PoolTrackerStats {
    pub total_pools: usize,
    pub pools_by_type: HashMap<PoolType, usize>,
    pub total_value_locked: U256,
}

impl PoolType {
    pub fn name(&self) -> &'static str {
        match self {
            PoolType::UniswapV2 => "UniswapV2",
            PoolType::UniswapV3 => "UniswapV3",
            PoolType::UniswapV4 => "UniswapV4",
            PoolType::Unknown => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pool_registration() {
        let tracker = PoolTracker::new(0.1);
        
        let pool_state = PoolState {
            pool_address: H160::random(),
            token0_address: H160::random(),
            token1_address: H160::random(),
            reserve0: U256::from(10u64) * U256::exp10(18), // 10 tokens
            reserve1: U256::from(5u64) * U256::exp10(18),  // 5 tokens
            total_supply: U256::from(100u64) * U256::exp10(18),
            block_number: 12345,
            timestamp: 1234567890,
            pool_type: PoolType::UniswapV2,
        };
        
        assert!(tracker.register_pool(pool_state.clone()));
        assert_eq!(tracker.get_pool(&pool_state.pool_address).unwrap().reserve0, pool_state.reserve0);
    }
    
    #[test]
    fn test_rug_pull_detection() {
        let tracker = PoolTracker::new(0.1);
        
        let pool_address = H160::random();
        let pool_state = PoolState {
            pool_address,
            token0_address: H160::random(),
            token1_address: H160::random(),
            reserve0: U256::from(100u64) * U256::exp10(18),
            reserve1: U256::from(50u64) * U256::exp10(18),
            total_supply: U256::from(1000u64) * U256::exp10(18),
            block_number: 12345,
            timestamp: 1234567890,
            pool_type: PoolType::UniswapV2,
        };
        
        tracker.register_pool(pool_state);
        
        // Test 95% liquidity removal - should trigger rug pull detection
        let new_reserves = (
            U256::from(5u64) * U256::exp10(18),  // 5% remaining
            U256::from(2u64) * U256::exp10(18)   // 4% remaining
        );
        
        assert!(tracker.detect_rug_pull(&pool_address, new_reserves));
    }
} 