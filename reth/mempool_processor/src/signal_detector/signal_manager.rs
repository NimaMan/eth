/// Signal Manager
/// 
/// Coordinates all signal detectors to analyze simulation results and emit signals.
/// This module acts as the main entry point for signal detection from state changes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::fs::OpenOptions;
use std::io::Write;
use std::str::FromStr;
use tracing::{info, debug, warn};
use alloy_primitives::Address;
use reth_tx_simulator::AddressStateChange;
use crate::token_tracking::TokenTrackingCache;
use crate::simulator::SimulationResult;
use crate::common::address::checksum_address;
use hex;

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
    signal_log_path: PathBuf,
}

impl SignalManager {
    /// Create a new signal manager
    pub fn new(config: SignalManagerConfig) -> Self {
        info!("🔍 Signal manager initialized");
        info!("📁 Using log directory: {}", config.log_dir.display());
        
        // Create log directory if it doesn't exist
        std::fs::create_dir_all(&config.log_dir).ok();
        
        // Create detector-specific log files
        let trading_log_path = config.log_dir.join("trading_enabled.log");
        let tax_log_path = config.log_dir.join("tax_signals.log");
        let signal_log_path = config.log_dir.join("signal_manager.log");
        
        // Create the signal manager log file with header
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&signal_log_path)
        {
            writeln!(file, "# Signal Manager Activity Log").ok();
            writeln!(file, "# Format: [timestamp] activity_type | details").ok();
            writeln!(file, "# ================================================").ok();
        }
        
        Self {
            config,
            liquidity_detector: LiquidityDetector::new(),
            stablecoin_detector: StablecoinDetector::new(),
            trading_status_detector: TradingStatusDetector::with_log_path(trading_log_path),
            tax_detector: TaxDetector::with_log_path(tax_log_path),
            token_cache: None,
            signal_log_path,
        }
    }
    
    /// Set the token tracking cache
    pub fn set_token_cache(&mut self, token_cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(token_cache.clone());
        self.liquidity_detector.set_token_cache(token_cache);
    }
    
    /// Log activity to the signal_manager.log file
    fn log_activity(&self, activity_type: &str, details: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(false)
            .append(true)
            .open(&self.signal_log_path)
        {
            let timestamp = chrono::Local::now();
            writeln!(file, "[{}] {} | {}", 
                timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                activity_type,
                details
            ).ok();
        }
    }
    
    /// Log a signal to the signal_manager.log file
    fn log_signal(&self, signal: &Signal) {
        if let Ok(mut file) = OpenOptions::new()
            .create(false)
            .append(true)
            .open(&self.signal_log_path)
        {
            let timestamp = chrono::Local::now();
            let log_entry = match signal {
                Signal::TradingEnabled(s) => {
                    format!("[{}] SIGNAL_DETECTED | TRADING_ENABLED | {} | token: {} | creator: {} | buy_tax: {}% | sell_tax: {}%",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        s.tx_hash,
                        s.token_address,
                        s.creator_address,
                        s.buy_tax,
                        s.sell_tax
                    )
                }
                Signal::HighTaxWarning(s) => {
                    let buy_tax_str = if s.buy_tax == 255 { 
                        "None".to_string() 
                    } else { 
                        format!("{}%", s.buy_tax) 
                    };
                    let sell_tax_str = if s.sell_tax == 255 { 
                        "None".to_string() 
                    } else { 
                        format!("{}%", s.sell_tax) 
                    };
                    
                    format!("[{}] SIGNAL_DETECTED | HIGH_TAX_WARNING | {} | token: {} | buy_tax: {} | sell_tax: {} | type: {:?}",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        s.tx_hash,
                        s.token_address,
                        buy_tax_str,
                        sell_tax_str,
                        s.warning_type
                    )
                }
                Signal::ScamDetection(s) => {
                    format!("[{}] SIGNAL_DETECTED | SCAM_DETECTION | {} | pool: {} | token: {} | scammer: {} | eth_drained: {:.4} | drain_%: {:.1}%",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        s.tx_hash,
                        s.pool_address,
                        s.token_address,
                        s.scammer_address,
                        s.eth_drained,
                        s.drain_percentage
                    )
                }
                _ => {
                    format!("[{}] SIGNAL_DETECTED | UNKNOWN_SIGNAL | {:?}",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        signal
                    )
                }
            };
            
            writeln!(file, "{}", log_entry).ok();
        }
    }
    
    /// Process simulation result to detect signals
    pub async fn process_simulation_result(
        &mut self,
        result: &SimulationResult,
    ) -> Vec<Signal> {
        info!("📨 Signal Manager: Received simulation result for TX {}", result.request.tx.hash);
        info!("  Simulation had error: {}", result.error.is_some());
        if let Some(ref bs) = result.buy_sell_result {
            info!("  Buy/Sell result: can_buy={}, can_sell={}", bs.can_buy, bs.can_sell);
        }
        
        // Log to signal_manager.log
        let error_msg = if let Some(ref err) = result.error {
            format!("Error: {}", err)
        } else {
            "Success".to_string()
        };
        
        // Add visual separator for new transaction
        self.log_activity("", "");  // Empty line
        self.log_activity("", &format!("════════════════════════════════════════════════════════════════════════════════"));
        self.log_activity("", &format!("TX: {}", result.request.tx.hash));
        
        self.log_activity("RECEIVED", &format!(
            "Category: {:?} | {} | BuySell: {}",
            result.request.category,
            error_msg,
            result.buy_sell_result.is_some()
        ));
        
        // Log creator token info if this is a creator transaction
        if let crate::tx_router::TransactionCategory::CreatorTransaction { creator, target_token, .. } = &result.request.category {
            if let Some(ref token_cache) = self.token_cache {
                // Get all tokens created by this creator - need to checksum the address
                let checksummed_creator = ethers::utils::to_checksum(&ethers::types::Address::from_str(creator).unwrap_or_default(), None);
                let creator_tokens = token_cache.get_tokens_by_creator(&checksummed_creator).await;
                if !creator_tokens.is_empty() {
                    let mut token_info_parts = Vec::new();
                    for token in &creator_tokens {
                        let pools = token_cache.get_pools_for_token(token).await;
                        let pool_count = pools.len();
                        let total_liquidity: f64 = pools.iter()
                            .map(|(_, pool)| pool.eth_reserve)
                            .sum();
                        token_info_parts.push(format!("{} (pools: {}, liquidity: {:.2} ETH)", token, pool_count, total_liquidity));
                    }
                    let token_info = token_info_parts.join(", ");
                    
                    self.log_activity("CREATOR_INFO", &format!(
                        "Creator: {} | Tokens: [{}] | Target: {:?}",
                        creator,
                        token_info,
                        target_token
                    ));
                } else {
                    self.log_activity("CREATOR_INFO", &format!(
                        "Creator: {} | No tokens tracked | Target: {:?}",
                        creator,
                        target_token
                    ));
                }
            }
        }
        
        let mut signals = Vec::new();
        
        // First, extract key values from simulation result
        let (_buy_tax, _sell_tax, _can_buy, _can_sell) = if let Some(buy_sell) = &result.buy_sell_result {
            self.log_activity("BUY_SELL_RESULT", &format!(
                "can_buy: {} | can_sell: {}",
                buy_sell.can_buy,
                buy_sell.can_sell
            ));
            
            (
                None::<f64>, // TODO: Calculate from state changes
                None::<f64>, // TODO: Calculate from state changes
                buy_sell.can_buy,
                buy_sell.can_sell,
            )
        } else {
            // No buy/sell simulation
            self.log_activity("NO_BUY_SELL", "No buy/sell result");
            
            // Still check pool state changes even without buy/sell simulation
            // This will detect liquidity removals
            (None::<f64>, None::<f64>, false, false)
        };
        
        // STEP 1: Tax detector (NOW ACTIVE) - Run first to get tax values
        let tax_signals = self.tax_detector.detect(result);
        
        // Always log tax detection results, even if no signals
        if let Some(ref buy_sell) = result.buy_sell_result {
            // Extract tax values from the detector results or buy/sell simulation
            let mut detected_buy_tax = 0.0;
            let mut detected_sell_tax = 0.0;
            
            // First check if tax signals were found and have values
            for tax_signal in &tax_signals {
                if let Some(buy_tax) = tax_signal.buy_tax {
                    detected_buy_tax = buy_tax;
                }
                if let Some(sell_tax) = tax_signal.sell_tax {
                    detected_sell_tax = sell_tax;
                }
            }
            
            // Log more detailed information about the detection
            // If can't buy/sell, show tax as None instead of 0%
            let buy_tax_str = if !buy_sell.can_buy {
                "None".to_string()
            } else if detected_buy_tax > 0.0 {
                format!("{:.1}%", detected_buy_tax)
            } else {
                "0%".to_string()
            };
            
            let sell_tax_str = if !buy_sell.can_sell {
                "None".to_string()
            } else if detected_sell_tax > 0.0 {
                format!("{:.1}%", detected_sell_tax)
            } else {
                "0%".to_string()
            };
            
            self.log_activity("TAX_RESULT", &format!(
                "Buy: {} | Sell: {} | Can Buy: {} | Can Sell: {} | Signals: {}",
                buy_tax_str,
                sell_tax_str,
                buy_sell.can_buy,
                buy_sell.can_sell,
                tax_signals.len()
            ));
        } else {
            self.log_activity("TAX_RESULT", &format!(
                "Found {} tax signals (no buy/sell result)",
                tax_signals.len()
            ));
        }
        
        // Extract tax values from tax signals for use in trading status
        let mut calculated_buy_tax = None;
        let mut calculated_sell_tax = None;
        
        for tax_signal in &tax_signals {
            // Store the tax values for trading status signal
            if tax_signal.buy_tax.is_some() || tax_signal.sell_tax.is_some() {
                calculated_buy_tax = tax_signal.buy_tax;
                calculated_sell_tax = tax_signal.sell_tax;
            }
            
            match tax_signal.signal_type {
                TaxSignalType::Honeypot => {
                    // Convert to high tax warning with honeypot type
                    // Use 255 to represent "None/Unknown" tax
                    let buy_tax_u8 = tax_signal.buy_tax
                        .map(|t| t.min(254.0) as u8)
                        .unwrap_or(255);
                    let sell_tax_u8 = tax_signal.sell_tax
                        .map(|t| t.min(254.0) as u8)
                        .unwrap_or(255);
                        
                    signals.push(Signal::HighTaxWarning(crate::signal_detector::HighTaxWarningSignal {
                        tx_hash: result.request.tx.hash.clone(),
                        token_address: tax_signal.token_address.clone(),
                        creator_address: None,
                        buy_tax: buy_tax_u8,
                        sell_tax: sell_tax_u8,
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
                        token_address: tax_signal.token_address.clone(),
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
        
        // STEP 2: Trading status detector (ACTIVE) - Now with tax values
        // Check for trading status changes
        if let Some(trading_signal) = self.trading_status_detector.detect(result) {
            // Always log what we detected
            let status_msg = match trading_signal.status_change {
                TradingStatusChange::TradingEnabled => "Trading Enabled",
                TradingStatusChange::TradingDisabled => "Trading Disabled",
                TradingStatusChange::TradingPaused => "Trading Paused",
                TradingStatusChange::NoChange => "No Change",
            };
            
            self.log_activity("TRADING_STATUS", &format!(
                "Status: {} | Can Trade: {} | Details: {}",
                status_msg,
                trading_signal.can_trade_after,
                trading_signal.details
            ));
            
            match trading_signal.status_change {
                TradingStatusChange::TradingEnabled => {
                    // Trading status detector will log to its own file
                    signals.push(Signal::TradingEnabled(crate::signal_detector::TradingEnabledSignal {
                        tx_hash: result.request.tx.hash.clone(),
                        token_address: trading_signal.token_address.clone(),
                        creator_address: trading_signal.executor.clone(),
                        buy_tax: calculated_buy_tax.unwrap_or(0.0) as u8,
                        sell_tax: calculated_sell_tax.unwrap_or(0.0) as u8,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                        block_number: 0,
                    }));
                }
                _ => {}
            }
        } else {
            // No trading signal detected - log what we found
            if let Some(ref buy_sell) = result.buy_sell_result {
                self.log_activity("TRADING_STATUS", &format!(
                    "No change | Can Buy: {} | Can Sell: {}",
                    buy_sell.can_buy,
                    buy_sell.can_sell
                ));
            } else {
                self.log_activity("TRADING_STATUS", "No change (no buy/sell result)");
            }
        }
        
        // STEP 3: Liquidity and Stablecoin detectors (currently disabled)
        // TODO: Enable after tax detector is verified working
        /*
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
        */
        
        // Log all detected signals
        for signal in &signals {
            self.log_signal(signal);
        }
        
        // STEP 4: Check for pool state changes in the transaction itself
        // Log any pools that are affected by this transaction
        if let Some(ref state_changes) = result.tx_state_changes {
            if let Some(ref token_cache) = self.token_cache {
                let mut pool_changes = Vec::new();
                
                // Check each address in the state changes
                for (address, state_change) in state_changes {
                    let address_bytes: &[u8] = address.as_ref();
                    let address_str = checksum_address(&hex::encode(address_bytes));
                    
                    // Check if this address is a tracked pool
                    if let Some(pool_state) = token_cache.get_pool_by_address(&address_str).await {
                        // Get ETH balance change from eth_net field
                        // eth_net is I256 (signed) in wei units
                        let eth_change_wei = state_change.eth_net;
                        // Convert to f64 in ETH units using to_string()
                        let eth_change = eth_change_wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                        
                        pool_changes.push(format!(
                            "Pool {} (token: {}) | ETH reserve: {:.4} → {:.4} (change: {:+.4})", 
                            address_str,
                            pool_state.token_address,
                            pool_state.eth_reserve,
                            pool_state.eth_reserve + eth_change,
                            eth_change
                        ));
                    }
                }
                
                // Log pool changes if any were found
                if !pool_changes.is_empty() {
                    self.log_activity("POOL_STATE_CHANGES", &format!(
                        "From: {} | Affected pools: {}",
                        checksum_address(&hex::encode(&result.request.tx.from)),
                        pool_changes.len()
                    ));
                    
                    for pool_change in &pool_changes {
                        self.log_activity("POOL_CHANGE_DETAIL", pool_change);
                    }
                    
                    // Check for liquidity removal - negative ETH change in pool
                    for (address, state_change) in state_changes {
                        let address_bytes: &[u8] = address.as_ref();
                        let address_str = format!("0x{}", hex::encode(address_bytes));
                        
                        if let Some(pool_state) = token_cache.get_pool_by_address(&address_str).await {
                            let eth_change_wei = state_change.eth_net;
                            let eth_change = eth_change_wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                            
                            // Check if pool will have less than 0.2 ETH after this transaction
                            let eth_remaining = pool_state.eth_reserve + eth_change;
                            if eth_remaining < 0.2 && eth_change < 0.0 { // Pool drained below 0.2 ETH
                                let drain_percentage = (eth_change.abs() / pool_state.eth_reserve) * 100.0;
                                
                                // Create scam detection signal for liquidity removal
                                signals.push(Signal::ScamDetection(crate::signal_detector::ScamDetectionSignal {
                                    tx_hash: result.request.tx.hash.clone(),
                                    pool_address: address_str.clone(),
                                    token_address: pool_state.token_address.clone(),
                                    scammer_address: checksum_address(&hex::encode(&result.request.tx.from)),
                                    eth_drained: eth_change.abs(),
                                    eth_remaining,
                                    drain_percentage,
                                    timestamp: chrono::Utc::now().timestamp() as u64,
                                    block_number: 0,
                                }));
                                
                                self.log_activity("LIQUIDITY_REMOVAL_DETECTED", &format!(
                                    "Pool: {} | ETH remaining: {:.4} | ETH drained: {:.4} | Drain %: {:.1}%",
                                    address_str,
                                    eth_remaining,
                                    eth_change.abs(),
                                    drain_percentage
                                ));
                                
                                self.log_signal(&signals[signals.len() - 1]);
                            }
                        }
                    }
                }
            }
        }
        
        // Also log a summary for this simulation
        if !signals.is_empty() {
            info!("📢 Detected {} signals for TX {}", signals.len(), result.request.tx.hash);
            self.log_activity("SIGNALS_SUMMARY", &format!(
                "Total signals detected: {}",
                signals.len()
            ));
        } else {
            self.log_activity("NO_SIGNALS", "No signals detected");
        }
        
        // Add closing separator
        self.log_activity("", &format!("══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════"));
        
        signals
    }
    
}