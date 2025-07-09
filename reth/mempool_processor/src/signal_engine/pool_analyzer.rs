// signal_engine/pool_analyzer.rs
//
// Pool state analysis module that analyzes address state changes
// to detect pool-level signals like liquidity drains, scams, etc.

use std::collections::HashMap;
use std::sync::Arc;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use tracing::{info, debug, warn};
use lazy_static::lazy_static;

use crate::pool_subscriber::cache::PoolStateCache;
use crate::common::address::alloy_address_to_checksum;
use super::types::{MarketEvent, EventType, Severity, EventMetrics, PoolEffect};

lazy_static! {
    /// Global pool analysis log file
    static ref POOL_ANALYSIS_LOG: Mutex<std::fs::File> = {
        let log_path = "logs/pool_analysis.log";
        if let Some(parent) = std::path::Path::new(log_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("Failed to open pool analysis log file");
        
        Mutex::new(file)
    };
    
    /// Global statistics for pool analysis
    static ref POOL_STATS: Mutex<PoolAnalysisStats> = Mutex::new(PoolAnalysisStats::default());
}

#[derive(Debug, Default)]
pub struct PoolAnalysisStats {
    pub total_pools_analyzed: u64,
    pub liquidity_drains_detected: u64,
    pub scams_detected: u64,
    pub liquidity_warnings: u64,
    pub pools_affected: HashMap<String, u64>,
}

/// Configuration for pool analysis
#[derive(Debug, Clone)]
pub struct PoolAnalysisConfig {
    /// ETH threshold below which pool is considered drained
    pub eth_threshold: f64,
    
    /// Percentage drain to trigger scam alert
    pub scam_drain_percent: f64,
    
    /// Percentage change to trigger liquidity warning
    pub liquidity_warning_percent: f64,
    
    /// Minimum confidence to report events
    pub min_confidence: f64,
}

impl Default for PoolAnalysisConfig {
    fn default() -> Self {
        Self {
            eth_threshold: 0.01,
            scam_drain_percent: 0.5,  // 50%
            liquidity_warning_percent: 0.2,  // 20%
            min_confidence: 0.7,
        }
    }
}

/// Pool state analyzer
pub struct PoolAnalyzer {
    pool_cache: Arc<PoolStateCache>,
    config: PoolAnalysisConfig,
}

impl PoolAnalyzer {
    pub fn new(pool_cache: Arc<PoolStateCache>, config: PoolAnalysisConfig) -> Self {
        info!("🏊 Pool analyzer initialized with thresholds: ETH={:.4}, Scam={}%, Warning={}%",
              config.eth_threshold, config.scam_drain_percent * 100.0, config.liquidity_warning_percent * 100.0);
        
        Self {
            pool_cache,
            config,
        }
    }
    
    /// Analyze address state changes to detect pool-level events
    pub fn analyze_state_changes(
        &self, 
        tx_hash: &str,
        state_changes: &HashMap<alloy_primitives::Address, crate::tx_simulator::AddressStateChange>
    ) -> Vec<MarketEvent> {
        let mut events = Vec::new();
        let mut stats = POOL_STATS.lock().unwrap();
        
        debug!("🔍 Analyzing {} address changes for tx {}", state_changes.len(), tx_hash);
        
        // Log analysis to file
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        if let Ok(mut log_file) = POOL_ANALYSIS_LOG.lock() {
            let _ = writeln!(log_file, "[{}] TX: {} | Addresses: {}", 
                           timestamp, tx_hash, state_changes.len());
        }
        
        // Check each address to see if it's a monitored pool
        for (address, changes) in state_changes {
            let address_str = alloy_address_to_checksum(*address);
            
            // Check if this is a monitored pool
            if let Some(pool_state) = self.pool_cache.get_pool(&address_str) {
                stats.total_pools_analyzed += 1;
                *stats.pools_affected.entry(address_str.clone()).or_insert(0) += 1;
                
                info!("🎯 Pool {} affected by tx {}", address_str, tx_hash);
                
                // Extract numeric values (now already f64)
                let eth_delta = changes.eth_net;
                let token_delta: f64 = changes.token_net.values().sum();
                
                // Calculate the effect on the pool
                let pool_effect = PoolEffect {
                    pool_address: address_str.clone(),
                    current_eth_reserve: pool_state.eth_reserve,
                    current_token_reserve: pool_state.token_reserve,
                    eth_delta,
                    token_delta,
                    simulated_eth_reserve: pool_state.eth_reserve + eth_delta,
                    simulated_token_reserve: pool_state.token_reserve + token_delta,
                    percentage_change: if pool_state.eth_reserve > 0.0 {
                        (eth_delta / pool_state.eth_reserve) * 100.0
                    } else {
                        0.0
                    },
                };
                
                // Log pool effect details
                if let Ok(mut log_file) = POOL_ANALYSIS_LOG.lock() {
                    let _ = writeln!(log_file, 
                        "[{}]   Pool: {} | ETH: {:.6} → {:.6} ({:+.6}, {:+.2}%) | Token: {:.6} → {:.6}",
                        timestamp, pool_effect.pool_address,
                        pool_effect.current_eth_reserve, pool_effect.simulated_eth_reserve,
                        pool_effect.eth_delta, pool_effect.percentage_change,
                        pool_effect.current_token_reserve, pool_effect.simulated_token_reserve
                    );
                }
                
                // Check for different event types
                if let Some(event) = self.check_scam_drain(&pool_effect, tx_hash, &pool_state.token_address) {
                    stats.scams_detected += 1;
                    events.push(event);
                }
                
                if let Some(event) = self.check_liquidity_warning(&pool_effect, tx_hash, &pool_state.token_address) {
                    stats.liquidity_warnings += 1;
                    events.push(event);
                }
            }
        }
        
        // Sort events by severity
        events.sort_by(|a, b| b.severity.cmp(&a.severity));
        
        debug!("🎯 Found {} events from pool analysis", events.len());
        events
    }
    
    /// Check for critical liquidity drain (scam)
    fn check_scam_drain(&self, effect: &PoolEffect, tx_hash: &str, token_address: &str) -> Option<MarketEvent> {
        // Skip if ETH is being added
        if effect.eth_delta >= 0.0 {
            return None;
        }
        
        let eth_drain_percent = effect.percentage_change.abs();
        let new_eth_reserve = effect.simulated_eth_reserve;
        
        // Check scam conditions
        let is_scam = eth_drain_percent >= self.config.scam_drain_percent * 100.0 ||
                     new_eth_reserve < self.config.eth_threshold;
        
        if is_scam {
            let confidence = if eth_drain_percent >= 90.0 { 0.95 } 
                           else if eth_drain_percent >= 70.0 { 0.90 }
                           else { 0.85 };
            
            if confidence >= self.config.min_confidence {
                warn!("🚨 SCAM DETECTED: Pool {} drained {:.2}% ({:.4} ETH remaining)",
                     effect.pool_address, eth_drain_percent, new_eth_reserve);
                
                // Log scam to file
                if let Ok(mut log_file) = POOL_ANALYSIS_LOG.lock() {
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let _ = writeln!(log_file,
                        "[{}]   🚨 SCAM ALERT: {:.2}% drain, {:.4} ETH remaining",
                        timestamp, eth_drain_percent, new_eth_reserve
                    );
                }
                
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
                        token_symbol: String::new(), // Would need token info
                        extra: HashMap::new(),
                    },
                    detection_time: chrono::Utc::now().timestamp() as f64,
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    block_number: 0, // Would need to pass this in
                    details: format!("Critical liquidity drain: {:.2}% of pool ETH removed", eth_drain_percent),
                });
            }
        }
        
        None
    }
    
    /// Check for significant liquidity changes
    fn check_liquidity_warning(&self, effect: &PoolEffect, tx_hash: &str, token_address: &str) -> Option<MarketEvent> {
        let change_percent = effect.percentage_change.abs();
        
        // Check if change exceeds warning threshold but not scam threshold
        if change_percent >= self.config.liquidity_warning_percent * 100.0 &&
           change_percent < self.config.scam_drain_percent * 100.0 {
            
            let confidence = 0.85;
            
            info!("⚠️ Liquidity warning: Pool {} changed by {:.2}%",
                 effect.pool_address, change_percent);
            
            return Some(MarketEvent {
                event_type: EventType::LiquidityWarning,
                severity: Severity::High,
                confidence,
                tx_hash: tx_hash.to_string(),
                pool_address: effect.pool_address.clone(),
                token_address: token_address.to_string(),
                metrics: EventMetrics {
                    eth_change: effect.eth_delta,
                    eth_percent: effect.percentage_change,
                    token_change: effect.token_delta,
                    token_percent: (effect.token_delta / effect.current_token_reserve) * 100.0,
                    new_eth_reserve: effect.simulated_eth_reserve,
                    new_token_reserve: effect.simulated_token_reserve,
                    token_symbol: String::new(),
                    extra: HashMap::new(),
                },
                detection_time: chrono::Utc::now().timestamp() as f64,
                timestamp: chrono::Utc::now().timestamp() as u64,
                block_number: 0, // Would need to pass this in
                details: format!("Significant liquidity change: {:.2}%", change_percent),
            });
        }
        
        None
    }
    
    /// Get current statistics
    pub fn get_stats(&self) -> PoolAnalysisStats {
        POOL_STATS.lock().unwrap().clone()
    }
    
    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = POOL_STATS.lock().unwrap();
        *stats = PoolAnalysisStats::default();
    }
}

impl Clone for PoolAnalysisStats {
    fn clone(&self) -> Self {
        Self {
            total_pools_analyzed: self.total_pools_analyzed,
            liquidity_drains_detected: self.liquidity_drains_detected,
            scams_detected: self.scams_detected,
            liquidity_warnings: self.liquidity_warnings,
            pools_affected: self.pools_affected.clone(),
        }
    }
}