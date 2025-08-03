/// Signal Manager
/// 
/// Coordinates all signal detectors to analyze simulation results and emit signals.
/// This module acts as the main entry point for signal detection from state changes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, debug, warn};
use alloy_primitives::Address;
use reth_tx_simulator::AddressStateChange;
use crate::token_tracking::TokenTrackingCache;
use crate::simulator::SimulationResult;

use super::{
    LiquidityDetector, LiquiditySignal,
    StablecoinDetector, StablecoinSignal,
    TradingStatusDetector, TradingStatusSignal,
    trading_status_detector::TradingStatusChange,
    TaxDetector, TaxSignal, TaxSignalType,
    Signal,
};

/// Configuration for signal detection
#[derive(Debug, Clone)]
pub struct SignalManagerConfig {
    /// Log directory for signal outputs
    pub log_dir: PathBuf,
}

impl Default for SignalManagerConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("logs/signals"),
        }
    }
}


/// Signal manager that coordinates all detectors
pub struct SignalManager {
    config: SignalManagerConfig,
    liquidity_detector: LiquidityDetector,
    stablecoin_detector: StablecoinDetector,
    trading_status_detector: TradingStatusDetector,
    tax_detector: TaxDetector,
    token_cache: Option<Arc<TokenTrackingCache>>,
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
            tax_detector: TaxDetector::new(),
            token_cache: None,
        }
    }
    
    /// Set the token tracking cache
    pub fn set_token_cache(&mut self, token_cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(token_cache.clone());
        self.liquidity_detector.set_token_cache(token_cache);
    }
    
    /// Process simulation result to detect signals
    pub async fn process_simulation_result(
        &mut self,
        result: &SimulationResult,
    ) -> Vec<Signal> {
        let mut signals = Vec::new();
        
        // First, extract key values from simulation result
        let (buy_tax, sell_tax, can_buy, can_sell) = if let Some(buy_sell) = &result.buy_sell_result {
            (
                None::<f64>, // TODO: Calculate from state changes
                None::<f64>, // TODO: Calculate from state changes
                buy_sell.can_buy,
                buy_sell.can_sell,
            )
        } else {
            // No buy/sell simulation means we can't analyze properly
            return signals;
        };
        
        // Use tax detector for all tax-related signals
        let tax_signals = self.tax_detector.detect(result);
        for tax_signal in tax_signals {
            match tax_signal.signal_type {
                TaxSignalType::Honeypot => {
                    // Convert to high tax warning with honeypot type
                    signals.push(Signal::HighTaxWarning(crate::signal_detector::HighTaxWarningSignal {
                        tx_hash: result.request.tx.hash.clone(),
                        token_address: tax_signal.token_address,
                        creator_address: None,
                        buy_tax: (tax_signal.buy_tax.unwrap_or(0.0)) as u8,
                        sell_tax: (tax_signal.sell_tax.unwrap_or(0.0)) as u8,
                        warning_type: crate::signal_detector::TaxWarningType::PotentialHoneypot,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                        block_number: 0,
                    }));
                }
                TaxSignalType::HighTax { buy, sell } => {
                    // Convert to high tax warning
                    let warning_type = if sell && tax_signal.sell_tax.unwrap_or(0.0) > 50.0 {
                        crate::signal_detector::TaxWarningType::PotentialHoneypot
                    } else if buy {
                        crate::signal_detector::TaxWarningType::HighBuyTax
                    } else {
                        crate::signal_detector::TaxWarningType::HighSellTax
                    };
                    
                    signals.push(Signal::HighTaxWarning(crate::signal_detector::HighTaxWarningSignal {
                        tx_hash: result.request.tx.hash.clone(),
                        token_address: tax_signal.token_address,
                        creator_address: None,
                        buy_tax: (tax_signal.buy_tax.unwrap_or(0.0)) as u8,
                        sell_tax: (tax_signal.sell_tax.unwrap_or(0.0)) as u8,
                        warning_type,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                        block_number: 0,
                    }));
                }
                _ => {
                    // Log other tax signals but don't convert to specific signal types yet
                    info!("💸 Tax signal detected: {:?}", tax_signal);
                }
            }
        }
        
        // Check for trading status changes
        if let Some(trading_signal) = self.trading_status_detector.detect(result) {
            match trading_signal.status_change {
                TradingStatusChange::TradingEnabled => {
                    // Check with tax detector if we should actually enable trading
                    let should_enable = self.tax_detector.should_enable_trading(
                        trading_signal.buy_tax,
                        trading_signal.sell_tax
                    );
                    
                    if should_enable {
                        info!("🎯 Trading newly enabled for token {}", trading_signal.token_address);
                        signals.push(Signal::TradingEnabled(crate::signal_detector::TradingEnabledSignal {
                            tx_hash: result.request.tx.hash.clone(),
                            token_address: trading_signal.token_address.clone(),
                            creator_address: trading_signal.executor.clone(),
                            buy_tax: (trading_signal.buy_tax.unwrap_or(0.0)) as u8,
                            sell_tax: (trading_signal.sell_tax.unwrap_or(0.0)) as u8,
                            timestamp: chrono::Utc::now().timestamp() as u64,
                            block_number: 0,
                        }));
                    } else {
                        warn!("⚠️  Trading enabled but taxes indicate honeypot for token {}", trading_signal.token_address);
                    }
                }
                _ => {}
            }
        }
        
        // Get state changes from buy/sell result for other detectors
        let state_changes = result.buy_sell_result.as_ref()
            .and_then(|bs| bs.buy_state_changes.as_ref());
        
        // Run liquidity detection if we have state changes
        if let Some(state_changes) = state_changes {
            let from_address = if let Ok(bytes) = hex::decode(&result.request.tx.from) {
                if let Ok(addr) = alloy_primitives::Address::try_from(bytes.as_slice()) {
                    addr
                } else {
                    return signals; // Invalid from address
                }
            } else {
                return signals; // Invalid from address hex
            };
            
            
            let liquidity_signals = self.liquidity_detector.detect(
                &result.request.tx.hash,
                from_address,
                state_changes,
            ).await;
            
            // Convert liquidity signals to the Signal enum
            for liq_signal in liquidity_signals {
                match liq_signal.signal_type {
                    super::LiquiditySignalType::ScamDetected => {
                        signals.push(Signal::ScamDetection(crate::signal_detector::ScamDetectionSignal {
                            tx_hash: liq_signal.tx_hash,
                            pool_address: liq_signal.pool_address,
                            token_address: liq_signal.token_address,
                            scammer_address: liq_signal.from_address,
                            eth_drained: liq_signal.eth_change.abs(),
                            eth_remaining: liq_signal.remaining_liquidity,
                            drain_percentage: liq_signal.percentage_change,
                            timestamp: chrono::Utc::now().timestamp() as u64,
                            block_number: 0,
                        }));
                    }
                    super::LiquiditySignalType::LiquidityRemoval => {
                        // Could add LiquidityRemoval signal if needed
                        info!("💧 Liquidity removal detected: {}", liq_signal.details);
                    }
                }
            }
            
            // Run stablecoin detection
            let to_address = result.request.tx.to.as_ref()
                .and_then(|to| hex::decode(to).ok())
                .and_then(|bytes| alloy_primitives::Address::try_from(bytes.as_slice()).ok());
            
            let stablecoin_signals = self.stablecoin_detector.detect(
                &result.request.tx.hash,
                from_address,
                to_address,
                state_changes,
            ).await;
            
            if !stablecoin_signals.is_empty() {
                info!("💰 Found {} stablecoin signals", stablecoin_signals.len());
            }
        }
        
        signals
    }
    
}