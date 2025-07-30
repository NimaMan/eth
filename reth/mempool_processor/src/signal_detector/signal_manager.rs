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
use crate::token_tracking::types::TokenInfo;
use crate::simulator::SimulationResult;

use super::{
    LiquidityDetector, LiquiditySignal,
    StablecoinDetector, StablecoinSignal,
    TradingStatusDetector, TradingStatusSignal, TradingStatusChange,
    Signal,
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
    trading_status_detector: TradingStatusDetector,
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
            trading_status_detector: TradingStatusDetector::new(),
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
    
    /// Process simulation result to detect signals
    pub async fn process_simulation_result(
        &mut self,
        result: &SimulationResult,
        token_info: Option<&TokenInfo>,
    ) -> Vec<Signal> {
        self.total_simulations_analyzed += 1;
        let mut signals = Vec::new();
        
        // Check for trading status changes
        if let Some(trading_signal) = self.trading_status_detector.detect(result) {
            // Context-aware detection
            if let Some(token_info) = token_info {
                match trading_signal.status_change {
                    TradingStatusChange::TradingEnabled => {
                        // Only emit signal if trading wasn't already enabled
                        if !token_info.trading_enabled {
                            info!("🎯 Trading newly enabled for token {}", trading_signal.token_address);
                            signals.push(Signal::TradingEnabled(crate::signal_detector::TradingEnabledSignal {
                                tx_hash: result.request.tx.hash.clone(),
                                token_address: trading_signal.token_address.clone(),
                                creator_address: trading_signal.executor.clone(),
                                buy_tax: (trading_signal.buy_tax.unwrap_or(0.0) * 100.0) as u8,
                                sell_tax: (trading_signal.sell_tax.unwrap_or(0.0) * 100.0) as u8,
                                timestamp: chrono::Utc::now().timestamp() as u64,
                                block_number: 0, // TODO: Get from result
                            }));
                        }
                    }
                    _ => {}
                }
                
                // Check for honeypot - was tradeable but now can't sell
                if token_info.trading_enabled && !trading_signal.can_trade_after {
                    warn!("🍯 Potential honeypot - trading disabled for {}", trading_signal.token_address);
                    signals.push(Signal::ScamDetection(crate::signal_detector::ScamDetectionSignal {
                        tx_hash: result.request.tx.hash.clone(),
                        pool_address: "".to_string(), // TODO: Get pool from token info
                        token_address: trading_signal.token_address.clone(),
                        scammer_address: trading_signal.executor.clone(),
                        eth_drained: 0.0,
                        scam_type: "honeypot_trading_disabled".to_string(),
                        confidence: 0.9,
                    }));
                }
            } else {
                // No context - for new tokens, just check if trading is enabled
                if trading_signal.status_change == TradingStatusChange::TradingEnabled {
                    signals.push(Signal::TradingEnabled(crate::signal_detector::TradingEnabledSignal {
                        tx_hash: result.request.tx.hash.clone(),
                        token_address: trading_signal.token_address.clone(),
                        creator_address: trading_signal.executor.clone(),
                        buy_tax: (trading_signal.buy_tax.unwrap_or(0.0) * 100.0) as u8,
                        sell_tax: (trading_signal.sell_tax.unwrap_or(0.0) * 100.0) as u8,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                        block_number: 0, // TODO: Get from result
                    }));
                }
            }
        }
        
        // Check for high tax warning
        if let Some(buy_sell) = &result.buy_sell_result {
            if let Some(buy_tax) = buy_sell.buy_tax {
                if let Some(sell_tax) = buy_sell.sell_tax {
                    if buy_tax > 0.25 || sell_tax > 0.25 {
                        signals.push(Signal::HighTaxWarning(crate::signal_detector::HighTaxWarningSignal {
                            tx_hash: result.request.tx.hash.clone(),
                            token_address: match &result.request.category {
                                crate::tx_router::TransactionCategory::ContractCreation { contract_address, .. } => contract_address.to_string(),
                                crate::tx_router::TransactionCategory::CreatorTransaction { token_address, .. } => token_address.clone(),
                                _ => "".to_string(),
                            },
                            creator_address: None,
                            buy_tax: (buy_tax * 100.0) as u8,
                            sell_tax: (sell_tax * 100.0) as u8,
                            warning_type: if sell_tax > 0.5 { 
                                crate::signal_detector::TaxWarningType::PotentialHoneypot 
                            } else if buy_tax > 0.25 {
                                crate::signal_detector::TaxWarningType::HighBuyTax
                            } else {
                                crate::signal_detector::TaxWarningType::HighSellTax 
                            },
                            timestamp: chrono::Utc::now().timestamp() as u64,
                            block_number: 0, // TODO: Get from result
                        }));
                    }
                }
            }
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