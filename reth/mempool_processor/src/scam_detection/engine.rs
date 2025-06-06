// scam_detection/engine.rs
//
// Implementation of the scam detection engine that compares simulated 
// transaction effects against current pool ETH levels to identify potential scams.
//
// Algorithm:
// 1. Receive a simulated transaction and its predicted state changes
// 2. For each affected pool, retrieve current pool state from PoolStateCache
// 3. Calculate the expected new ETH reserve after transaction execution
// 4. Apply detection rules to identify suspicious transactions:
//    - If ETH reserve would drop below configured threshold
//    - If large percentage of ETH reserves would be removed
//    - If transaction matches known scam patterns
// 5. Generate and publish alerts for suspicious transactions

use std::sync::Arc;
use tracing::{info, debug, warn};
use ethers::types::H256;

use crate::pool_subscriber::cache::PoolStateCache;
use super::types::{ScamAlert, ScamAlertReason, SimulationResult, PoolEffect};

/// Configurable parameters for scam detection
#[derive(Debug, Clone)]
pub struct ScamDetectionConfig {
    /// Minimum ETH reserve threshold (in ETH)
    pub eth_threshold: f64,
    
    /// Percentage threshold for large withdrawals (0.0-1.0)
    pub percentage_threshold: f64,
}

impl Default for ScamDetectionConfig {
    fn default() -> Self {
        Self {
            eth_threshold: 0.15,
            percentage_threshold: 0.5, // 50%
        }
    }
}

/// The ScamDetectionEngine analyzes simulated transaction effects against
/// current pool levels to identify potential scam transactions.
pub struct ScamDetectionEngine {
    /// The pool state cache provides current ETH reserves
    pool_cache: Arc<PoolStateCache>,
    
    /// Configuration parameters
    config: ScamDetectionConfig,
}

impl ScamDetectionEngine {
    /// Create a new ScamDetectionEngine with the specified pool cache and config
    pub fn new(pool_cache: Arc<PoolStateCache>, config: ScamDetectionConfig) -> Self {
        Self {
            pool_cache,
            config,
        }
    }
    
    /// Create a new ScamDetectionEngine with the specified pool cache and default config
    pub fn with_pool_cache(pool_cache: Arc<PoolStateCache>) -> Self {
        Self::new(pool_cache, ScamDetectionConfig::default())
    }
    
    /// Analyze a simulated transaction to detect potential scams
    /// Returns a vector of ScamAlert for suspicious transactions
    pub fn analyze_transaction(&self, simulation: SimulationResult) -> Vec<ScamAlert> {
        let mut alerts = Vec::new();
        
        debug!("Analyzing tx {} from {} affecting {} pools", 
              simulation.tx_hash, simulation.from, simulation.affected_pools.len());
              
        // For each affected pool, check if it would be scammed
        for (pool_address, effect) in simulation.affected_pools.iter() {
            // Skip pools where ETH is being added (positive delta)
            if effect.eth_delta >= 0.0 {
                continue;
            }
            
            // Get the current pool state from cache
            if let Some(pool_state) = self.pool_cache.get_pool(pool_address) {
                // Check if pool state is fresh (less than 60 seconds old)
                const MAX_POOL_AGE: std::time::Duration = std::time::Duration::from_secs(60);
                if pool_state.is_stale(MAX_POOL_AGE) {
                    warn!("Pool state for {} is stale ({:.1}s old), may produce inaccurate results", 
                         pool_address, pool_state.age().as_secs_f64());
                }
                
                // Dynamic threshold calculation based on pool size
                let dynamic_eth_threshold = if effect.current_eth_reserve < 1.0 {
                    // For very small pools, use percentage-based detection only
                    0.0
                } else if effect.current_eth_reserve < 5.0 {
                    // For small pools, scale the threshold
                    self.config.eth_threshold * (effect.current_eth_reserve / 5.0)
                } else {
                    // For larger pools, use the configured threshold
                    self.config.eth_threshold
                };
                
                // Check if pool would drop below dynamic threshold while currently above it
                let would_drain_pool = effect.simulated_eth_reserve < dynamic_eth_threshold 
                    && effect.current_eth_reserve >= dynamic_eth_threshold
                    && effect.current_eth_reserve >= 0.5; // Only flag if pool had reasonable liquidity
                
                // Debug log for significant ETH changes in pools
                if effect.eth_delta.abs() > 0.1 {
                    debug!("📊 Pool {} ETH change: {:.6} → {:.6} (Δ{:.6}) | Dynamic threshold: {:.3} | Would drain: {}", 
                           pool_address, effect.current_eth_reserve, effect.simulated_eth_reserve, 
                           effect.eth_delta, dynamic_eth_threshold, would_drain_pool);
                }
                
                // Calculate if this is a large percentage withdrawal
                // For small pools, be more tolerant of percentage changes
                let percentage_threshold = if effect.current_eth_reserve < 0.5 {
                    0.8 // 80% threshold for very small pools
                } else if effect.current_eth_reserve < 2.0 {
                    0.7 // 70% threshold for small pools
                } else {
                    self.config.percentage_threshold // Default 50% for normal pools
                };
                
                let large_withdrawal = effect.percentage_change.abs() > percentage_threshold;
                
                // If either condition is met, create an alert
                if would_drain_pool || large_withdrawal {
                    let reason = if would_drain_pool {
                        ScamAlertReason::EthReserveDepleted
                    } else {
                        ScamAlertReason::LargeEthWithdrawal
                    };
                    
                    let alert = ScamAlert {
                        tx_hash: simulation.tx_hash.to_string(),
                        from_address: simulation.from.clone(),
                        pool_address: pool_address.clone(),
                        token_address: pool_state.token_address.clone(),
                        current_eth_reserve: effect.current_eth_reserve,
                        simulated_eth_reserve: effect.simulated_eth_reserve,
                        reason,
                        eth_threshold: self.config.eth_threshold,
                        detection_block: pool_state.last_updated_block,
                        detection_time: chrono::Utc::now().timestamp() as f64,
                    };
                    
                    info!("SCAM ALERT: Detected potential scam in tx {}: {} -> {} ETH in pool {}",
                         simulation.tx_hash, effect.current_eth_reserve, 
                         effect.simulated_eth_reserve, pool_address);
                    
                    alerts.push(alert);
                }
            } else {
                warn!("Pool {} affected by tx {} not found in cache", 
                     pool_address, simulation.tx_hash);
            }
        }
        
        alerts
    }
    
    /// Process a transaction by address and simulated ETH effect on pools
    /// This is a simplified version for initial testing that doesn't require
    /// the full SimulationResult, just the essential details
    pub fn process_transaction(
        &self, 
        tx_hash: H256, 
        from_address: String,
        affected_pools: Vec<(String, f64, f64)> // (pool_address, current_eth, new_eth)
    ) -> Vec<ScamAlert> {
        // Convert the simplified input to a SimulationResult
        let mut simulation_pools = std::collections::HashMap::new();
        
        for (pool_address, current_eth, new_eth) in affected_pools {
            let eth_delta = new_eth - current_eth;
            let percentage_change = if current_eth > 0.0 {
                eth_delta / current_eth
            } else {
                0.0
            };
            
            let effect = PoolEffect {
                pool_address: pool_address.clone(),
                current_eth_reserve: current_eth,
                simulated_eth_reserve: new_eth,
                eth_delta,
                percentage_change,
            };
            
            simulation_pools.insert(pool_address, effect);
        }
        
        let simulation = SimulationResult {
            tx_hash,
            from: from_address,
            affected_pools: simulation_pools,
        };
        
        self.analyze_transaction(simulation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::pool_subscriber::types::PoolUpdate;
    
    // Helper to create a test pool cache with sample data
    fn create_test_pool_cache() -> Arc<PoolStateCache> {
        let cache = PoolStateCache::new(0.05);
        
        // Add some test pools
        let mut updates = HashMap::new();
        
        // Pool with healthy reserves
        updates.insert(
            "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            PoolUpdate {
                eth_reserve: 10.0,
                token_address: "0xtoken1".to_string(),
                block_number: 12345,
                update_time: 1626000000.0,
            }
        );
        
        // Pool with low reserves
        updates.insert(
            "0x2234567890abcdef1234567890abcdef12345678".to_string(),
            PoolUpdate {
                eth_reserve: 0.1,
                token_address: "0xtoken2".to_string(),
                block_number: 12345,
                update_time: 1626000000.0,
            }
        );
        
        cache.update_pools(updates.iter());
        Arc::new(cache)
    }
    
    #[test]
    fn test_detect_eth_reserve_depletion() {
        let pool_cache = create_test_pool_cache();
        let engine = ScamDetectionEngine::with_pool_cache(pool_cache);
        
        // Test a transaction that would deplete the low reserve pool
        let tx_hash = H256::random();
        let alerts = engine.process_transaction(
            tx_hash,
            "0xsender".to_string(),
            vec![
                // Format: (pool_address, current_eth, new_eth)
                ("0x2234567890abcdef1234567890abcdef12345678".to_string(), 0.1, 0.01)
            ]
        );
        
        // Should detect one scam
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].reason, ScamAlertReason::EthReserveDepleted);
    }
    
    #[test]
    fn test_detect_large_withdrawal() {
        let pool_cache = create_test_pool_cache();
        let engine = ScamDetectionEngine::with_pool_cache(pool_cache);
        
        // Test a transaction that would withdraw 80% from a healthy pool
        let tx_hash = H256::random();
        let alerts = engine.process_transaction(
            tx_hash,
            "0xsender".to_string(),
            vec![
                // Format: (pool_address, current_eth, new_eth)
                ("0x1234567890abcdef1234567890abcdef12345678".to_string(), 10.0, 2.0)
            ]
        );
        
        // Should detect one scam
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].reason, ScamAlertReason::LargeEthWithdrawal);
    }
    
    #[test]
    fn test_no_alert_for_safe_transaction() {
        let pool_cache = create_test_pool_cache();
        let engine = ScamDetectionEngine::with_pool_cache(pool_cache);
        
        // Test a transaction that would make a small withdrawal
        let tx_hash = H256::random();
        let alerts = engine.process_transaction(
            tx_hash,
            "0xsender".to_string(),
            vec![
                // Format: (pool_address, current_eth, new_eth)
                ("0x1234567890abcdef1234567890abcdef12345678".to_string(), 10.0, 9.0)
            ]
        );
        
        // Should not detect any scams
        assert_eq!(alerts.len(), 0);
    }
} 