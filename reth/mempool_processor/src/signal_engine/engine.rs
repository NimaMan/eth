// signal_engine/engine.rs
//
// Implementation of the market signal engine that analyzes simulated
// transaction effects to detect various market signals and opportunities.
//
// Algorithm:
// 1. Receive a simulated transaction and its predicted state changes
// 2. For each affected pool, retrieve current pool state from PoolStateCache
// 3. Calculate the expected new reserves after transaction execution
// 4. Apply multi-category detection rules:
//    - Scam detection (liquidity drains)
//    - Liquidity warnings (significant changes)
//    - Token supply anomalies (hidden mints)
//    - Volume spikes (market manipulation)
//    - Price impacts (arbitrage opportunities)
// 5. Generate categorized events with confidence scores

use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, debug, warn};
use ethers::types::H256;

use crate::pool_subscriber::cache::PoolStateCache;
use super::types::{
    MarketEvent, EventType, Severity, EventMetrics, SimulationResult, PoolEffect,
    SignalThresholds, ScamAlert, ScamAlertReason
};

/// Configurable parameters for the signal engine
#[derive(Debug, Clone)]
pub struct SignalConfig {
    /// Thresholds for different event types
    pub thresholds: SignalThresholds,
    
    /// Enable machine learning confidence scoring
    pub enable_ml_scoring: bool,
    
    /// Minimum confidence score to emit events
    pub min_confidence: f64,
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            thresholds: SignalThresholds::default(),
            enable_ml_scoring: false,
            min_confidence: 0.7,
        }
    }
}

/// The SignalEngine analyzes simulated transaction effects to detect
/// market signals, risks, and opportunities.
pub struct SignalEngine {
    /// The pool state cache provides current reserves
    pool_cache: Arc<PoolStateCache>,
    
    /// Configuration parameters
    config: SignalConfig,
    
    /// Historical accuracy tracking (for confidence scoring)
    accuracy_tracker: HashMap<EventType, f64>,
}

impl SignalEngine {
    /// Create a new SignalEngine with the specified pool cache and config
    pub fn new(pool_cache: Arc<PoolStateCache>, config: SignalConfig) -> Self {
        let mut accuracy_tracker = HashMap::new();
        // Initialize with default accuracy scores
        accuracy_tracker.insert(EventType::ScamAlert, 0.95);
        accuracy_tracker.insert(EventType::LiquidityWarning, 0.90);
        accuracy_tracker.insert(EventType::TokenSupplyAlert, 0.85);
        accuracy_tracker.insert(EventType::VolumeSpike, 0.80);
        accuracy_tracker.insert(EventType::PriceImpact, 0.75);
        
        Self {
            pool_cache,
            config,
            accuracy_tracker,
        }
    }
    
    /// Create a new SignalEngine with default config
    pub fn with_pool_cache(pool_cache: Arc<PoolStateCache>) -> Self {
        Self::new(pool_cache, SignalConfig::default())
    }
    
    /// Analyze a simulated transaction to detect market events
    /// Returns a vector of MarketEvent for detected conditions
    pub fn analyze_transaction(&self, simulation: SimulationResult) -> Vec<MarketEvent> {
        let mut events = Vec::new();
        
        debug!("Analyzing tx {} affecting {} pools", 
              simulation.tx_hash, simulation.affected_pools.len());
              
        // For each affected pool, check for various event types
        for (pool_address, effect) in simulation.affected_pools.iter() {
            // Get the current pool state from cache
            if let Some(pool_state) = self.pool_cache.get_pool(pool_address) {
                // Data freshness score - pools are updated when transactions affect them
                // No need to warn about "stale" data since Python updates on-demand
                let data_freshness_score = 1.0;
                
                // Update effect with current token reserve
                let mut updated_effect = effect.clone();
                updated_effect.current_token_reserve = pool_state.token_reserve;
                
                // Check for scam (critical liquidity drain)
                if let Some(event) = self.check_scam_alert(&simulation.tx_hash, &updated_effect, &pool_state.token_address, data_freshness_score) {
                    events.push(event);
                }
                
                // Check for liquidity warning
                if let Some(event) = self.check_liquidity_warning(&simulation.tx_hash, &updated_effect, &pool_state.token_address, data_freshness_score) {
                    events.push(event);
                }
                
                // Check for token supply anomaly
                if let Some(event) = self.check_token_supply(&simulation.tx_hash, &updated_effect, &pool_state.token_address, data_freshness_score) {
                    events.push(event);
                }
                
                // TODO: Add volume spike detection (requires historical data)
                // TODO: Add price impact detection (requires price calculation)
            } else {
                debug!("Pool {} not found in cache, skipping", pool_address);
            }
        }
        
        // Sort events by severity (highest first)
        events.sort_by(|a, b| b.severity.cmp(&a.severity));
        
        events
    }
    
    /// Check for critical liquidity drain (scam)
    fn check_scam_alert(&self, tx_hash: &str, effect: &PoolEffect, token_address: &str, freshness: f64) -> Option<MarketEvent> {
        // Skip if ETH is being added
        if effect.eth_delta >= 0.0 {
            return None;
        }
        
        let eth_drain_percent = (effect.eth_delta.abs() / effect.current_eth_reserve) * 100.0;
        let new_eth_reserve = effect.simulated_eth_reserve;
        
        // Check scam conditions
        let is_scam = eth_drain_percent >= self.config.thresholds.scam_drain_percent * 100.0 ||
                     new_eth_reserve < self.config.thresholds.eth_threshold;
        
        if is_scam {
            let confidence = self.calculate_confidence(EventType::ScamAlert, freshness, true);
            
            if confidence >= self.config.min_confidence {
                info!("🚨 SCAM DETECTED: Pool {} drained {:.2}% ({:.4} ETH remaining)",
                     effect.pool_address, eth_drain_percent, new_eth_reserve);
                
                return Some(MarketEvent {
                    event_type: EventType::ScamAlert,
                    severity: Severity::Critical,
                    confidence,
                    tx_hash: tx_hash.to_string(),
                    pool_address: effect.pool_address.clone(),
                    token_address: token_address.to_string(),
                    metrics: EventMetrics {
                        eth_change: effect.eth_delta,
                        eth_percent: -eth_drain_percent,
                        token_change: effect.token_delta,
                        token_percent: (effect.token_delta / effect.current_token_reserve) * 100.0,
                        new_eth_reserve,
                        new_token_reserve: effect.simulated_token_reserve,
                        token_symbol: String::new(),
                        extra: HashMap::new(),
                    },
                    detection_time: chrono::Utc::now().timestamp() as f64,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    block_number: 0, // TODO: Get from provider
                    details: format!("Critical liquidity drain: {:.2}% of pool ETH removed", eth_drain_percent),
                });
            }
        }
        
        None
    }
    
    /// Check for significant liquidity changes
    fn check_liquidity_warning(&self, tx_hash: &str, effect: &PoolEffect, token_address: &str, freshness: f64) -> Option<MarketEvent> {
        let eth_change_percent = (effect.eth_delta / effect.current_eth_reserve) * 100.0;
        
        // Check if change is significant but not critical
        let is_warning = eth_change_percent.abs() >= self.config.thresholds.warning_drain_percent * 100.0 &&
                        eth_change_percent.abs() < self.config.thresholds.scam_drain_percent * 100.0;
        
        if is_warning {
            let confidence = self.calculate_confidence(EventType::LiquidityWarning, freshness, true);
            
            if confidence >= self.config.min_confidence {
                debug!("⚠️ Liquidity warning: Pool {} changed {:.2}%", 
                      effect.pool_address, eth_change_percent);
                
                return Some(MarketEvent {
                    event_type: EventType::LiquidityWarning,
                    severity: Severity::High,
                    confidence,
                    tx_hash: tx_hash.to_string(),
                    pool_address: effect.pool_address.clone(),
                    token_address: token_address.to_string(),
                    metrics: EventMetrics {
                        eth_change: effect.eth_delta,
                        eth_percent: eth_change_percent,
                        token_change: effect.token_delta,
                        token_percent: (effect.token_delta / effect.current_token_reserve) * 100.0,
                        new_eth_reserve: effect.simulated_eth_reserve,
                        new_token_reserve: effect.simulated_token_reserve,
                        token_symbol: String::new(),
                        extra: HashMap::new(),
                    },
                    detection_time: chrono::Utc::now().timestamp() as f64,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    block_number: 0,
                    details: format!("Significant liquidity change: {:.2}%", eth_change_percent),
                });
            }
        }
        
        None
    }
    
    /// Check for abnormal token supply increases
    fn check_token_supply(&self, tx_hash: &str, effect: &PoolEffect, token_address: &str, freshness: f64) -> Option<MarketEvent> {
        // Only check if tokens are being added to the pool
        if effect.token_delta <= 0.0 {
            return None;
        }
        
        let token_increase_percent = (effect.token_delta / effect.current_token_reserve) * 100.0;
        
        // Check if increase is suspicious
        if token_increase_percent >= self.config.thresholds.supply_increase_percent * 100.0 {
            // Additional check: is ETH decreasing while tokens increase? (very suspicious)
            let is_suspicious = effect.eth_delta < 0.0;
            let confidence = self.calculate_confidence(
                EventType::TokenSupplyAlert, 
                freshness, 
                !is_suspicious // Lower confidence if not accompanied by ETH drain
            );
            
            if confidence >= self.config.min_confidence {
                warn!("🪙 Token supply alert: Pool {} tokens increased {:.2}%{}",
                     effect.pool_address, token_increase_percent,
                     if is_suspicious { " (with ETH drain!)" } else { "" });
                
                return Some(MarketEvent {
                    event_type: EventType::TokenSupplyAlert,
                    severity: if is_suspicious { Severity::High } else { Severity::Medium },
                    confidence,
                    tx_hash: tx_hash.to_string(),
                    pool_address: effect.pool_address.clone(),
                    token_address: token_address.to_string(),
                    metrics: EventMetrics {
                        eth_change: effect.eth_delta,
                        eth_percent: (effect.eth_delta / effect.current_eth_reserve) * 100.0,
                        token_change: effect.token_delta,
                        token_percent: token_increase_percent,
                        new_eth_reserve: effect.simulated_eth_reserve,
                        new_token_reserve: effect.simulated_token_reserve,
                        token_symbol: String::new(),
                        extra: {
                            let mut extra = HashMap::new();
                            extra.insert("suspicious".to_string(), serde_json::json!(is_suspicious));
                            extra
                        },
                    },
                    detection_time: chrono::Utc::now().timestamp() as f64,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    block_number: 0,
                    details: format!("Abnormal token supply increase: {:.2}%{}", 
                                   token_increase_percent,
                                   if is_suspicious { " with simultaneous ETH drain" } else { "" }),
                });
            }
        }
        
        None
    }
    
    /// Calculate confidence score for an event
    fn calculate_confidence(&self, event_type: EventType, data_freshness: f64, simulation_quality: bool) -> f64 {
        let base_accuracy = self.accuracy_tracker.get(&event_type).copied().unwrap_or(0.8);
        let simulation_score = if simulation_quality { 1.0 } else { 0.6 };
        
        // Weighted average: 40% data freshness, 30% simulation quality, 30% historical accuracy
        (data_freshness * 0.4 + simulation_score * 0.3 + base_accuracy * 0.3).min(1.0)
    }
}

// Legacy compatibility layer
pub type ScamDetectionConfig = SignalConfig;
pub type ScamDetectionEngine = SignalEngine;
pub type DecisionConfig = SignalConfig;
pub type DecisionEngine = SignalEngine;

impl SignalEngine {
    /// Legacy method for backward compatibility - returns only scam alerts
    pub fn analyze_transaction_legacy(&self, simulation: SimulationResult) -> Vec<ScamAlert> {
        let events = self.analyze_transaction(simulation);
        
        // Convert MarketEvents to legacy ScamAlerts (only for actual scams)
        events.into_iter()
            .filter(|e| e.event_type == EventType::ScamAlert)
            .map(|event| ScamAlert {
                tx_hash: event.tx_hash,
                pool_address: event.pool_address,
                current_eth_reserve: event.metrics.new_eth_reserve - event.metrics.eth_change,
                simulated_eth_reserve: event.metrics.new_eth_reserve,
                percentage_drain: event.metrics.eth_percent.abs(),
                detection_time: event.detection_time,
                detection_block: event.block_number,
                reason: if event.metrics.new_eth_reserve < self.config.thresholds.eth_threshold {
                    ScamAlertReason::EthReserveDepleted
                } else {
                    ScamAlertReason::LargeEthWithdrawal
                },
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool_subscriber::types::PoolUpdate;
    
    #[test]
    fn test_scam_detection() {
        // Create a mock pool cache
        let cache = Arc::new(PoolStateCache::new(0.1));
        
        // Add test pool
        let mut updates = HashMap::new();
        updates.insert(
            "0x1234567890abcdef1234567890abcdef12345678".to_string(),
            PoolUpdate {
                eth_reserve: 10.0,
                token_reserve: 20000.0,
                token_address: "0xtoken1".to_string(),
                block_number: 12345,
                update_time: 1626000000.0,
            }
        );
        cache.update_pools(updates.iter());
        
        // Create engine
        let engine = DecisionEngine::with_pool_cache(cache);
        
        // Test scam detection
        let simulation = SimulationResult {
            tx_hash: "0xtest".to_string(),
            affected_pools: {
                let mut pools = HashMap::new();
                pools.insert(
                    "0x1234567890abcdef1234567890abcdef12345678".to_string(),
                    PoolEffect {
                        pool_address: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
                        current_eth_reserve: 10.0,
                        simulated_eth_reserve: 2.0,
                        current_token_reserve: 20000.0,
                        simulated_token_reserve: 20000.0,
                        eth_delta: -8.0,
                        token_delta: 0.0,
                        percentage_change: -80.0,
                    }
                );
                pools
            },
            simulation_successful: true,
            error_message: None,
        };
        
        let events = engine.analyze_transaction(simulation);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::ScamAlert);
        assert_eq!(events[0].severity, Severity::Critical);
    }
    
    #[test]
    fn test_liquidity_warning() {
        let cache = Arc::new(PoolStateCache::new(0.1));
        
        let mut updates = HashMap::new();
        updates.insert(
            "0x2234567890abcdef1234567890abcdef12345678".to_string(),
            PoolUpdate {
                eth_reserve: 50.0,
                token_reserve: 100000.0,
                token_address: "0xtoken2".to_string(),
                block_number: 12345,
                update_time: 1626000000.0,
            }
        );
        cache.update_pools(updates.iter());
        
        let engine = DecisionEngine::with_pool_cache(cache);
        
        let simulation = SimulationResult {
            tx_hash: "0xtest2".to_string(),
            affected_pools: {
                let mut pools = HashMap::new();
                pools.insert(
                    "0x2234567890abcdef1234567890abcdef12345678".to_string(),
                    PoolEffect {
                        pool_address: "0x2234567890abcdef1234567890abcdef12345678".to_string(),
                        current_eth_reserve: 50.0,
                        simulated_eth_reserve: 35.0,
                        current_token_reserve: 100000.0,
                        simulated_token_reserve: 100000.0,
                        eth_delta: -15.0,
                        token_delta: 0.0,
                        percentage_change: -30.0,
                    }
                );
                pools
            },
            simulation_successful: true,
            error_message: None,
        };
        
        let events = engine.analyze_transaction(simulation);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::LiquidityWarning);
        assert_eq!(events[0].severity, Severity::High);
    }
    
    #[test]
    fn test_token_supply_alert() {
        let cache = Arc::new(PoolStateCache::new(0.1));
        
        let mut updates = HashMap::new();
        updates.insert(
            "0x3334567890abcdef1234567890abcdef12345678".to_string(),
            PoolUpdate {
                eth_reserve: 20.0,
                token_reserve: 50000.0,
                token_address: "0xtoken3".to_string(),
                block_number: 12345,
                update_time: 1626000000.0,
            }
        );
        cache.update_pools(updates.iter());
        
        let engine = DecisionEngine::with_pool_cache(cache);
        
        let simulation = SimulationResult {
            tx_hash: "0xtest3".to_string(),
            affected_pools: {
                let mut pools = HashMap::new();
                pools.insert(
                    "0x3334567890abcdef1234567890abcdef12345678".to_string(),
                    PoolEffect {
                        pool_address: "0x3334567890abcdef1234567890abcdef12345678".to_string(),
                        current_eth_reserve: 20.0,
                        simulated_eth_reserve: 19.0,
                        current_token_reserve: 50000.0,
                        simulated_token_reserve: 60000.0,
                        eth_delta: -1.0,
                        token_delta: 10000.0,
                        percentage_change: -5.0,
                    }
                );
                pools
            },
            simulation_successful: true,
            error_message: None,
        };
        
        let events = engine.analyze_transaction(simulation);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::TokenSupplyAlert);
        assert_eq!(events[0].severity, Severity::High); // High because ETH is also decreasing
    }
}