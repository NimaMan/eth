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
use tracing::{info, debug, warn, error};
use alloy_primitives::Address;
use reth_tx_simulator::AddressStateChange;
use tokio::sync::Mutex;
use crate::token_tracking::TokenTrackingCache;
use crate::simulator::SimulationResult;
use crate::common::address::checksum_address;
use crate::signal_publisher::SignalPublisher;
use hex;

use super::{
    LiquidityDetector, LiquiditySignal,
    StablecoinDetector, StablecoinSignal,
    TradingStatusDetector, TradingStatusSignal,
    trading_status_detector::TradingStatusChange,
    TaxDetector, TaxSignal, TaxSignalType,
    LpApprovalDetector, LpApprovalSignal,
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
    lp_approval_detector: LpApprovalDetector,
    token_cache: Option<Arc<TokenTrackingCache>>,
    signal_log_path: PathBuf,
    publisher: Option<Arc<Mutex<SignalPublisher>>>,
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
            lp_approval_detector: LpApprovalDetector::new(),
            token_cache: None,
            signal_log_path,
            publisher: None,
        }
    }
    
    /// Set the token tracking cache
    pub fn set_token_cache(&mut self, token_cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(token_cache.clone());
        self.liquidity_detector.set_token_cache(token_cache);
    }
    
    /// Set the signal publisher
    pub fn set_publisher(&mut self, publisher: Arc<Mutex<SignalPublisher>>) {
        self.publisher = Some(publisher);
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
        
        // STEP 3: Liquidity detector - Check for pool drains and liquidity removals
        // Use tx_state_changes which has the actual transaction state changes
        if let Some(ref state_changes) = result.tx_state_changes {
            // tx.from is already bytes (Vec<u8>), no need to decode
            let from_address_result = alloy_primitives::Address::try_from(result.request.tx.from.as_slice()).ok();
            
            if let Some(from_address) = from_address_result {
            
            // Run liquidity detection
            let liquidity_signals = self.liquidity_detector.detect(
                &result.request.tx.hash,
                from_address,
                state_changes,
            ).await;
            
            // Convert liquidity signals to the Signal enum
            for liq_signal in liquidity_signals {
                match liq_signal.signal_type {
                    super::liquidity_detector::SignalType::ScamDetected => {
                        let scam_signal = Signal::ScamDetection(crate::signal_detector::ScamDetectionSignal {
                            tx_hash: liq_signal.tx_hash.clone(),
                            pool_address: liq_signal.pool_address.clone(),
                            token_address: liq_signal.token_address.clone(),
                            scammer_address: liq_signal.from_address.clone(),
                            eth_drained: liq_signal.eth_change.abs(),
                            eth_remaining: liq_signal.remaining_liquidity,
                            drain_percentage: liq_signal.percentage_change,
                            timestamp: chrono::Utc::now().timestamp() as u64,
                            block_number: 0,
                        });
                        
                        self.log_activity("LIQUIDITY_SCAM_DETECTED", &format!(
                            "Pool: {} | ETH drained: {:.4} | Remaining: {:.4} | Drain %: {:.1}%",
                            liq_signal.pool_address,
                            liq_signal.eth_change.abs(),
                            liq_signal.remaining_liquidity,
                            liq_signal.percentage_change
                        ));
                        
                        signals.push(scam_signal);
                    }
                    super::liquidity_detector::SignalType::LiquidityRemoval => {
                        // Log liquidity removal but don't create a signal yet
                        self.log_activity("LIQUIDITY_REMOVAL", &format!(
                            "Pool: {} | ETH removed: {:.4} | Remaining: {:.4} | Type: {:?}",
                            liq_signal.pool_address,
                            liq_signal.eth_change.abs(),
                            liq_signal.remaining_liquidity,
                            liq_signal.change_type
                        ));
                        info!("💧 Liquidity removal detected: {}", liq_signal.details);
                    }
                }
            }
            } else {
                // Log that we couldn't parse the from address
                self.log_activity("LIQUIDITY_ERROR", "Invalid from address - skipping liquidity detection");
            }
        }
        
        // Log all detected signals
        for signal in &signals {
            self.log_signal(signal);
        }
        
        // STEP 4: Log pool state changes for debugging (detection is done by LiquidityDetector)
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
                        let eth_change_wei = state_change.eth_net;
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
                
                // Log pool changes if any were found (for debugging only)
                if !pool_changes.is_empty() {
                    self.log_activity("POOL_STATE_CHANGES", &format!(
                        "From: {} | Affected pools: {}",
                        checksum_address(&hex::encode(&result.request.tx.from)),
                        pool_changes.len()
                    ));
                    
                    for pool_change in &pool_changes {
                        self.log_activity("POOL_CHANGE_DETAIL", pool_change);
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
            
            // Publish all detected signals immediately
            if let Some(ref publisher) = self.publisher {
                let mut pub_guard = publisher.lock().await;
                for signal in &signals {
                    if let Err(e) = pub_guard.publish(signal.clone()).await {
                        error!("Failed to publish signal: {}", e);
                    }
                }
                info!("✅ Published {} signals", signals.len());
            }
        } else {
            self.log_activity("NO_SIGNALS", "No signals detected");
        }
        signals
    }
    
    /// Detect LP approval signals from non-simulated transactions
    pub async fn detect_lp_approval(
        &mut self,
        tx: &crate::mempool_fetcher::MempoolTransaction,
        category: &crate::tx_router::TransactionCategory,
    ) {
        // Log the transaction receipt
        self.log_activity("LP_APPROVAL_CHECK", &format!("TX: {}", tx.hash));
        
        // Check for LP approval
        if let Some(lp_signal) = self.lp_approval_detector.detect_from_transaction(tx, category) {
            self.log_activity("LP_APPROVAL_DETECTED", &format!(
                "Creator: {} | LP Token: {} | Router: {} | Amount: {}",
                lp_signal.creator,
                lp_signal.lp_token_address,
                lp_signal.router_address,
                lp_signal.amount
            ));
            
            // Log the critical warning
            warn!("🚨🚨🚨 RUG PULL SETUP DETECTED 🚨🚨🚨");
            warn!("Transaction: {}", lp_signal.tx_hash);
            warn!("Creator {} is preparing to remove liquidity!", lp_signal.creator);
            
            // Publish the signal immediately if we have a publisher
            if let Some(ref publisher) = self.publisher {
                // Create a scam detection signal for LP approval (rug pull setup)
                let signal = Signal::ScamDetection(crate::signal_detector::ScamDetectionSignal {
                    tx_hash: lp_signal.tx_hash,
                    pool_address: lp_signal.lp_token_address.clone(),
                    token_address: lp_signal.lp_token_address,
                    scammer_address: lp_signal.creator,
                    eth_drained: 0.0, // Not drained yet, just approved
                    eth_remaining: 0.0, // Unknown until actual removal
                    drain_percentage: 0.0, // Will be 100% when executed
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    block_number: 0,
                });
                
                // Log the signal
                self.log_signal(&signal);
                
                // Publish it
                let mut pub_guard = publisher.lock().await;
                if let Err(e) = pub_guard.publish(signal).await {
                    error!("Failed to publish LP approval signal: {}", e);
                }
            }
        } else {
            self.log_activity("LP_APPROVAL_CHECK", "Not an LP approval");
        }
    }
    
}