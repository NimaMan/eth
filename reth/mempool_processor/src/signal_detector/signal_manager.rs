/// Signal Manager
/// 
/// Coordinates all signal detectors to analyze simulation results and emit signals.
/// This module acts as the main entry point for signal detection from state changes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, debug};
use alloy_primitives::Address;
use reth_tx_simulator::AddressStateChange;
use crate::token_tracking::cache::PoolStateCache;

use super::{
    LiquidityDetector, LiquiditySignal,
    StablecoinDetector, StablecoinSignal,
    // Other detectors can be added here
};

/// Configuration for signal detection
#[derive(Debug, Clone)]
pub struct SignalManagerConfig {
    /// Log directory for signal outputs
    pub log_dir: PathBuf,
    /// Enable liquidity/scam detection
    pub enable_liquidity_detection: bool,
    /// Enable stablecoin activity detection
    pub enable_stablecoin_detection: bool,
}

impl Default for SignalManagerConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("logs/signals"),
            enable_liquidity_detection: true,
            enable_stablecoin_detection: true,
        }
    }
}

/// Aggregated signals from all detectors
#[derive(Debug, Clone)]
pub struct DetectedSignals {
    pub liquidity_signals: Vec<LiquiditySignal>,
    pub stablecoin_signals: Vec<StablecoinSignal>,
    // Other signal types can be added here
}

/// Signal manager that coordinates all detectors
pub struct SignalManager {
    config: SignalManagerConfig,
    liquidity_detector: LiquidityDetector,
    stablecoin_detector: StablecoinDetector,
    // Stats tracking
    total_simulations_analyzed: u64,
}

impl SignalManager {
    /// Create a new signal manager
    pub fn new(config: SignalManagerConfig) -> Self {
        info!("🔍 Signal manager initialized");
        info!("📁 Using log directory: {}", config.log_dir.display());
        
        // Create log directory if it doesn't exist
        std::fs::create_dir_all(&config.log_dir).ok();
        
        Self {
            config,
            liquidity_detector: LiquidityDetector::new(),
            stablecoin_detector: StablecoinDetector::new(),
            total_simulations_analyzed: 0,
        }
    }
    
    /// Set the pool state cache for detectors that need it
    pub fn set_pool_cache(&mut self, pool_cache: PoolStateCache) {
        self.liquidity_detector.set_pool_cache(pool_cache);
    }
    
    /// Analyze state changes from a transaction simulation
    pub async fn analyze_simulation_result(
        &mut self,
        tx_hash: &str,
        from_address: Address,
        to_address: Option<Address>,
        state_changes: &HashMap<Address, AddressStateChange>,
        simulation_time_us: u64,
    ) -> DetectedSignals {
        self.total_simulations_analyzed += 1;
        
        debug!("🔍 Analyzing {} address changes for tx {}", state_changes.len(), tx_hash);
        
        let mut signals = DetectedSignals {
            liquidity_signals: Vec::new(),
            stablecoin_signals: Vec::new(),
        };
        
        // Run liquidity/scam detection
        if self.config.enable_liquidity_detection {
            let liquidity_signals = self.liquidity_detector.detect(
                tx_hash,
                from_address,
                state_changes,
            ).await;
            
            if !liquidity_signals.is_empty() {
                info!("💧 Found {} liquidity signals for tx {}", liquidity_signals.len(), tx_hash);
            }
            
            signals.liquidity_signals = liquidity_signals;
        }
        
        // Run stablecoin detection
        if self.config.enable_stablecoin_detection {
            let stablecoin_signals = self.stablecoin_detector.detect(
                tx_hash,
                from_address,
                to_address,
                state_changes,
            ).await;
            
            if !stablecoin_signals.is_empty() {
                info!("💰 Found {} stablecoin signals for tx {}", stablecoin_signals.len(), tx_hash);
            }
            
            signals.stablecoin_signals = stablecoin_signals;
        }
        
        // Log summary
        let total_signals = signals.liquidity_signals.len() + signals.stablecoin_signals.len();
        if total_signals > 0 {
            info!("🎯 Found {} total signals from simulation of {} ({}μs)", 
                  total_signals, tx_hash, simulation_time_us);
        }
        
        signals
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> SignalManagerStats {
        SignalManagerStats {
            total_simulations_analyzed: self.total_simulations_analyzed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SignalManagerStats {
    pub total_simulations_analyzed: u64,
}