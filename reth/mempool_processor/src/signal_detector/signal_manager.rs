/// Signal Manager
/// 
/// Coordinates all signal detectors to analyze simulation results and emit signals.
/// 
/// KEY ARCHITECTURE - PER-POOL SIGNAL GENERATION:
/// - Receives SimulationResult for EACH pool independently
/// - Each signal is a function of (token_address, pool_address)
/// - A token with 3 pools generates 3 separate signals
/// - Each signal contains pool-specific data:
///   * pool_address: Unique identifier for the pool
///   * pool_type: V2, V3, or V4
///   * Tax values specific to that pool
///   * Liquidity metrics for that pool
/// 
/// Signal Types (all per-pool):
/// - TradingEnabled: Trading activated on a specific pool
/// - HighTaxWarning: High taxes detected on a specific pool
/// - ScamDetection: Liquidity drain from a specific pool
/// - LiquidityRemoval: Liquidity removed from a specific pool

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
use crate::config::TaxDetectionConfig;
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
    /// Minimum ETH liquidity to consider trading already enabled (default: 0.5 ETH)
    pub min_liquidity_threshold: f64,
    /// Tax detection configuration
    pub tax_detection: TaxDetectionConfig,
}

impl Default for SignalManagerConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("logs/signals"),
            min_liquidity_threshold: 0.5, // 0.5 ETH minimum liquidity
            tax_detection: TaxDetectionConfig::default(),
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
            config: config.clone(),
            liquidity_detector: LiquidityDetector::new(),
            stablecoin_detector: StablecoinDetector::new(),
            trading_status_detector: TradingStatusDetector::with_config(trading_log_path, config.min_liquidity_threshold),
            tax_detector: TaxDetector::with_log_path(config.tax_detection.clone(), tax_log_path),
            lp_approval_detector: LpApprovalDetector::new(),
            token_cache: None,
            signal_log_path,
            publisher: None,
        }
    }
    
    /// Set the token tracking cache
    pub fn set_token_cache(&mut self, token_cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(token_cache.clone());
        self.liquidity_detector.set_token_cache(token_cache.clone());
        self.trading_status_detector.set_token_cache(token_cache.clone());
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
                Signal::TaxSignal(s) => {
                    let buy_tax_str = s.buy_tax
                        .map(|tax| format!("{:.1}%", tax))
                        .unwrap_or_else(|| "None".to_string());
                    let sell_tax_str = s.sell_tax
                        .map(|tax| format!("{:.1}%", tax))
                        .unwrap_or_else(|| "None".to_string());
                    
                    format!("[{}] SIGNAL_DETECTED | TAX_SIGNAL | {} | token: {} | pool: {} | buy_tax: {} | sell_tax: {} | type: {}",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        s.tx_hash,
                        s.token_address,
                        s.pool_address,
                        buy_tax_str,
                        sell_tax_str,
                        s.signal_type
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
    /// 
    /// CRITICAL: This function is called ONCE PER POOL
    /// - Each pool's SimulationResult is processed independently
    /// - Generates signals specific to the (token, pool) pair
    /// - Pool address and type are extracted from the result
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
            // Check if it's the expected "not implemented" message
            if err.contains("Contract creation simulation not implemented") {
                format!("Warning: {}", err)
            } else {
                format!("Error: {}", err)
            }
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
        if let crate::tx_router::TransactionCategory::CreatorTransaction { creator, target_token, target_address, function_type, .. } = &result.request.category {
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
                    
                    // Check if this is an approve on a pool/LP token
                    if matches!(function_type, crate::function_detector::CreatorFunctionType::Other(s) if s == "approve") {
                        // Check if target_address matches any of the pools
                        for token in &creator_tokens {
                            let pools = token_cache.get_pools_for_token(token).await;
                            for (pool_addr, pool_state) in pools {
                                if target_address.eq_ignore_ascii_case(&pool_addr) {
                                    self.log_activity("LP_TOKEN_APPROVAL", &format!(
                                        "🚨 Creator approving LP tokens! | Pool: {} | Token: {} | Liquidity: {:.2} ETH",
                                        pool_addr,
                                        token,
                                        pool_state.eth_reserve
                                    ));
                                    break;
                                }
                            }
                        }
                    }
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
            
            // Include pool type if available
            let pool_type_str = result.pool_type.as_ref()
                .map(|pt| format!(" | Pool: {}", pt))
                .unwrap_or_else(|| "".to_string());
            
            self.log_activity("TAX_RESULT", &format!(
                "Buy: {} | Sell: {} | Can Buy: {} | Can Sell: {} | Signals: {}{}",
                buy_tax_str,
                sell_tax_str,
                buy_sell.can_buy,
                buy_sell.can_sell,
                tax_signals.len(),
                pool_type_str
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
                TaxSignalType::HighTaxOrHoneypot { cant_sell, buy_tax_exceeds_threshold, sell_tax_exceeds_threshold } => {
                    // Create pool-specific tax signal
                    let pool_address = result.pool_address
                        .map(|addr| format!("0x{}", hex::encode(addr)))
                        .unwrap_or_else(|| "unknown".to_string());
                    let pool_type = result.pool_type.clone()
                        .unwrap_or_else(|| "V2".to_string());
                    
                    // Extract creator from category
                    let creator_address = match &result.request.category {
                        crate::tx_router::TransactionCategory::CreatorTransaction { creator, .. } => creator.clone(),
                        _ => "unknown".to_string(),
                    };
                    
                    signals.push(Signal::TaxSignal(crate::signal_detector::types::TaxSignalRecord {
                        tx_hash: result.request.tx.hash.clone(),
                        token_address: tax_signal.token_address.clone(),
                        pool_address,
                        pool_type,
                        creator_address,
                        signal_type: "HighTaxOrHoneypot".to_string(),
                        signal_details: tax_signal.details.clone(),
                        confidence: tax_signal.confidence,
                        buy_tax: tax_signal.buy_tax,
                        sell_tax: tax_signal.sell_tax,
                        buy_tax_exceeds_threshold,
                        sell_tax_exceeds_threshold,
                        cant_sell,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    }));
                }
                _ => {
                    // Log other tax signals but don't convert to specific signal types yet
                    info!("💸 Tax signal detected: {:?}", tax_signal);
                }
            }
        }
        
        // STEP 2: Trading status detector (ACTIVE) - Now with tax values
        // Check for trading status changes or detect already-enabled trading
        if let Some(trading_signal) = self.trading_status_detector.detect(result, calculated_buy_tax, calculated_sell_tax).await {
            // Always log what we detected
            let status_msg = match trading_signal.status_change {
                TradingStatusChange::TradingEnabled => {
                    // Check if this is a detection of already-enabled trading
                    if trading_signal.details.contains("ALREADY ENABLED") {
                        "Trading Detected (Already Enabled)"
                    } else {
                        "Trading Enabled"
                    }
                },
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
                    // CRITICAL: Create pool-specific trading enabled signal
                    // Each pool gets its own signal with unique pool_address
                    let pool_address = result.pool_address
                        .map(|addr| format!("0x{}", hex::encode(addr)))
                        .unwrap_or_else(|| "unknown".to_string());
                    let pool_type = result.pool_type.clone()
                        .unwrap_or_else(|| "V2".to_string());
                    
                    signals.push(Signal::TradingEnabled(crate::signal_detector::TradingEnabledSignal {
                        tx_hash: result.request.tx.hash.clone(),
                        token_address: trading_signal.token_address.clone(),
                        pool_address,  // Now includes the specific pool
                        pool_type,     // Pool type (V2, V3, V4)
                        creator_address: trading_signal.executor.clone(),
                        buy_tax: calculated_buy_tax.unwrap_or(0.0) as u8,
                        sell_tax: calculated_sell_tax.unwrap_or(0.0) as u8,
                        timestamp: chrono::Utc::now().timestamp() as u64,
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
                        // Pool-specific scam detection (liquidity drain from THIS pool)
                        let scam_signal = Signal::ScamDetection(crate::signal_detector::ScamDetectionSignal {
                            tx_hash: liq_signal.tx_hash.clone(),
                            pool_address: liq_signal.pool_address.clone(),
                            pool_type: liq_signal.pool_type.clone(),  // Include pool type
                            token_address: liq_signal.token_address.clone(),
                            scammer_address: liq_signal.from_address.clone(),
                            eth_drained: liq_signal.eth_change.abs(),
                            eth_remaining: liq_signal.remaining_liquidity,
                            drain_percentage: liq_signal.percentage_change,
                            timestamp: chrono::Utc::now().timestamp() as u64,
                        });
                        
                        self.log_activity("LIQUIDITY_SCAM_DETECTED", &format!(
                            "Pool: {} | Type: {} | ETH drained: {:.4} | Remaining: {:.4} | Drain %: {:.1}%",
                            liq_signal.pool_address,
                            liq_signal.pool_type,
                            liq_signal.eth_change.abs(),
                            liq_signal.remaining_liquidity,
                            liq_signal.percentage_change
                        ));
                        
                        signals.push(scam_signal);
                    }
                    super::liquidity_detector::SignalType::LiquidityRemoval => {
                        // Log liquidity removal but don't create a signal yet
                        self.log_activity("LIQUIDITY_REMOVAL", &format!(
                            "Pool: {} | Type: {} | ETH removed: {:.4} | Remaining: {:.4} | Change: {:?}",
                            liq_signal.pool_address,
                            liq_signal.pool_type,
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
                // Get pool type from cache if available
                let pool_type = if let Some(ref token_cache) = self.token_cache {
                    if let Some(pool_state) = token_cache.get_pool_by_address(&lp_signal.lp_token_address).await {
                        pool_state.pool_type.clone()
                    } else {
                        "V2".to_string() // Default to V2 if unknown
                    }
                } else {
                    "V2".to_string()
                };
                
                // Create a scam detection signal for LP approval (rug pull setup)
                let signal = Signal::ScamDetection(crate::signal_detector::ScamDetectionSignal {
                    tx_hash: lp_signal.tx_hash,
                    pool_address: lp_signal.lp_token_address.clone(),
                    pool_type,  // Include pool type
                    token_address: lp_signal.lp_token_address,
                    scammer_address: lp_signal.creator,
                    eth_drained: 0.0, // Not drained yet, just approved
                    eth_remaining: 0.0, // Unknown until actual removal
                    drain_percentage: 0.0, // Will be 100% when executed
                    timestamp: chrono::Utc::now().timestamp() as u64,
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