use crate::config::TaxDetectionConfig;
use crate::signal_publisher::SignalPublisher;
use crate::simulator::SimulationResult;
use crate::token_tracking::TokenTrackingCache;
use alloy_primitives::{Address, U256};
use reth_chain_query::to_checksum_address;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use super::{
    trading_status_detector::TradingStatusChange, LiquidityDetector, LpApprovalDetector, Signal,
    TaxDetector, TaxSignalType, TradingStatusDetector,
};

/// Configuration for signal detection
#[derive(Debug, Clone)]
pub struct SignalManagerConfig {
    /// Log directory for signal outputs
    pub log_dir: PathBuf,
    /// Tax detection configuration
    pub tax_detection: TaxDetectionConfig,
}

impl Default for SignalManagerConfig {
    fn default() -> Self {
        // Create a timestamped directory for test/example runs
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
        let log_dir = PathBuf::from(format!("logs/test_signals_{}", timestamp));

        Self {
            log_dir,
            tax_detection: TaxDetectionConfig::default(),
        }
    }
}

/// Signal manager that coordinates all detectors
pub struct SignalManager {
    _config: SignalManagerConfig,
    liquidity_detector: LiquidityDetector,
    trading_status_detector: TradingStatusDetector,
    tax_signal_detector: TaxDetector,
    lp_approval_detector: LpApprovalDetector,
    token_cache: Option<Arc<TokenTrackingCache>>,
    signal_log_path: PathBuf,
    error_log_path: PathBuf,
    publisher: Option<Arc<Mutex<SignalPublisher>>>,
    total_signals_emitted: u64,
}

impl SignalManager {
    /// Create a new signal manager
    pub fn new(config: SignalManagerConfig) -> Self {
        info!("🔍 Signal manager initialized");
        info!("📁 Using log directory: {}", config.log_dir.display());

        // Create log directory if it doesn't exist
        std::fs::create_dir_all(&config.log_dir).ok();

        // Create detector-specific log files
        let simulation_results_log_path = config.log_dir.join("simulation_results.log");
        let tax_log_path = config.log_dir.join("tax_signals.log");
        let signal_log_path = config.log_dir.join("signal_manager.log");
        let error_log_path = config
            .log_dir
            .parent()
            .map(|p| p.join("simulation_errors.log"))
            .unwrap_or_else(|| config.log_dir.join("simulation_errors.log"));

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
        // Create unified error log file (sim + signal issues)
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&error_log_path)
        {
            writeln!(file, "# Simulation/Signal Errors/Warnings").ok();
            writeln!(file, "# Format: [timestamp] level | details").ok();
            writeln!(file, "# ==================================").ok();
        }

        Self {
            _config: config.clone(),
            liquidity_detector: LiquidityDetector::new(),
            trading_status_detector: TradingStatusDetector::with_log_path(
                simulation_results_log_path,
            ),
            tax_signal_detector: TaxDetector::with_log_path(
                config.tax_detection.clone(),
                tax_log_path,
            ),
            lp_approval_detector: LpApprovalDetector::new(&config.log_dir),
            token_cache: None,
            signal_log_path,
            error_log_path,
            publisher: None,
            total_signals_emitted: 0,
        }
    }

    /// Set the token tracking cache
    pub fn set_token_cache(&mut self, token_cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(token_cache.clone());
        self.liquidity_detector.set_token_cache(token_cache.clone());
        self.trading_status_detector
            .set_token_cache(token_cache.clone());
    }

    /// Set the signal publisher
    pub fn set_publisher(&mut self, publisher: Arc<Mutex<SignalPublisher>>) {
        self.publisher = Some(publisher);
    }

    /// Log activity to the signal_manager.log file
    fn log_activity(&self, activity_type: &str, details: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.signal_log_path)
        {
            let timestamp = chrono::Local::now();
            writeln!(
                file,
                "[{}] {} | {}",
                timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                activity_type,
                details
            )
            .ok();
        }
    }

    fn log_error(&self, level: &str, details: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.error_log_path)
        {
            let timestamp = chrono::Local::now();
            writeln!(
                file,
                "[{}] {} | {}",
                timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                level,
                details
            )
            .ok();
        }
    }

    fn log_sim_error(&self, level: &str, details: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.error_log_path)
        {
            let timestamp = chrono::Local::now();
            writeln!(
                file,
                "[{}] {} | {}",
                timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                level,
                details
            )
            .ok();
        }
    }

    /// Log a signal to the signal_manager.log file
    fn log_signal(&self, signal: &Signal) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.signal_log_path)
        {
            let timestamp = chrono::Local::now();
            let log_entry = match signal {
                Signal::LpApproval(_) => return,
                Signal::TradingEnabled(s) => {
                    format!(
                        "[{}] SIGNAL_DETECTED | TRADING_ENABLED | {} | token: {} | pool: {} | pool_type: {} | creator: {} | buy_tax: {}% | sell_tax: {}%",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        s.tx_hash,
                        s.token_address,
                        s.pool_address,
                        s.pool_type,
                        s.creator_address,
                        s.buy_tax,
                        s.sell_tax
                    )
                }
                Signal::TaxSignal(s) => {
                    let buy_tax_str = s
                        .buy_tax
                        .map(|tax| format!("{:.1}%", tax))
                        .unwrap_or_else(|| "None".to_string());
                    let sell_tax_str = s
                        .sell_tax
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
                Signal::LiquidityRemoval(s) => {
                    let est_eth = s
                        .estimated_eth_removed
                        .map(|v| format!("{:.4}", v))
                        .unwrap_or_else(|| "unknown".to_string());
                    format!(
                        "[{}] SIGNAL_DETECTED | LIQUIDITY_REMOVAL | {} | pool: {} | pool_type: {} | remover: {} | est_removed_eth: {} | function: {}",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        s.tx_hash,
                        s.pool_address,
                        s.pool_type,
                        s.remover_address,
                        est_eth,
                        s.function_name
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
    pub async fn process_simulation_result(&mut self, result: &SimulationResult) -> Vec<Signal> {
        if let Some(ref err) = result.error {
            if !err.contains("No pools found for token") {
                self.log_sim_error("ERROR", &format!("{} | {}", result.request.tx.hash, err));
                self.log_error("ERROR", &format!("{} | {}", result.request.tx.hash, err));
            }
        }

        // Skip logging for contract creation transactions to reduce noise
        if !matches!(
            result.request.category,
            crate::tx_router::TransactionCategory::ContractCreation { .. }
        ) {
            // Compact per‑TX header (written only to signal_manager.log, not console)
            self.log_activity("TX", &result.request.tx.hash);
            self.log_activity(
                "RECEIVED",
                &format!("Category: {:?}", result.request.category),
            );
        }

        // Log creator token info if this is a creator transaction
        if let crate::tx_router::TransactionCategory::CreatorTransaction {
            creator,
            target_token: _,
            target_address,
            function_type,
            ..
        } = &result.request.category
        {
            if let Some(ref token_cache) = self.token_cache {
                // Get all tokens created by this creator.
                let checksummed_creator = creator
                    .parse::<Address>()
                    .map(|address| to_checksum_address(&address))
                    .unwrap_or_else(|_| creator.clone());
                let creator_tokens = token_cache
                    .get_tokens_by_creator(&checksummed_creator)
                    .await;
                if !creator_tokens.is_empty() {
                    let mut token_info_parts = Vec::new();
                    for token in &creator_tokens {
                        let pools = token_cache.get_pools_for_token(&token.address).await;
                        let pool_count = pools.len();
                        let total_liquidity: f64 = pools.iter().map(|pool| pool.eth_reserve).sum();
                        token_info_parts.push(format!(
                            "{} (pools: {}, liquidity: {:.2} ETH)",
                            token.address, pool_count, total_liquidity
                        ));
                    }
                    // Trim: omit verbose creator/token listing

                    // Check if this is an approve on a pool/LP token
                    if matches!(function_type, crate::function_detector::CreatorFunctionType::Other(s) if s == "approve")
                    {
                        // Check if target_address matches any of the pools
                        for token in &creator_tokens {
                            let pools = token_cache.get_pools_for_token(&token.address).await;
                            for pool_state in pools {
                                if target_address.eq_ignore_ascii_case(&pool_state.address) {
                                    // Trim: rely on LP_APPROVAL signal logging
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut signals = Vec::new();

        // First, extract key values from simulation result
        let (_buy_tax, _sell_tax, _can_buy, _can_sell) =
            if let Some(buy_sell) = &result.buy_sell_result() {
                self.log_activity(
                    "BUY_SELL_RESULT",
                    &format!(
                        "can_buy: {} | can_sell: {}",
                        buy_sell.can_buy, buy_sell.can_sell
                    ),
                );

                (
                    None::<f64>, // TODO: Calculate from state changes
                    None::<f64>, // TODO: Calculate from state changes
                    buy_sell.can_buy,
                    buy_sell.can_sell,
                )
            } else {
                // No buy/sell simulation
                (None::<f64>, None::<f64>, false, false)
            };

        // STEP 1: Tax detector (NOW ACTIVE) - Run to get tax signals
        let tax_signals = self.tax_signal_detector.detect(result);

        // Extract tax values from simulation result (calculated in simulation_manager)
        let (calculated_buy_tax, calculated_sell_tax) =
            if let Some(ref buy_sell) = result.buy_sell_result() {
                (buy_sell.buy_tax, buy_sell.sell_tax)
            } else {
                (None, None)
            };

        // Trim: omit granular TAX_RESULT lines in signal_manager.log

        // Process tax signals to create actual signal records
        for tax_signal in &tax_signals {
            match tax_signal.signal_type {
                TaxSignalType::HighTaxOrHoneypot {
                    cant_sell,
                    buy_tax_exceeds_threshold,
                    sell_tax_exceeds_threshold,
                } => {
                    // Check trading status from token cache before logging TAX_SIGNAL
                    let should_log_tax_signal = if let Some(ref token_cache) = self.token_cache {
                        // Check if trading is enabled on any pool for this token
                        let pools = token_cache
                            .get_pools_for_token(&tax_signal.token_address)
                            .await;
                        let trading_enabled = pools.iter().any(|p| p.trading_enabled);

                        if trading_enabled {
                            // Trading is enabled - always log TAX_SIGNAL
                            true
                        } else {
                            // Trading is disabled - only log if we can buy/sell (potential TRADING_ENABLED signal)
                            if let Some(ref bs) = result.buy_sell_result() {
                                bs.can_buy || bs.can_sell
                            } else {
                                // Can't determine buy/sell capability - skip
                                false
                            }
                        }
                    } else {
                        // No token cache - log all TAX_SIGNALS
                        true
                    };

                    if should_log_tax_signal {
                        // Create pool-specific tax signal
                        let pool_address = result
                            .pool_address
                            .map(|addr| to_checksum_address(&addr))
                            .unwrap_or_else(|| "unknown".to_string());
                        let pool_type =
                            result.pool_type.clone().unwrap_or_else(|| "V2".to_string());

                        // Extract creator from category
                        let creator_address = match &result.request.category {
                            crate::tx_router::TransactionCategory::CreatorTransaction {
                                creator,
                                ..
                            } => creator.clone(),
                            _ => "unknown".to_string(),
                        };

                        signals.push(Signal::TaxSignal(
                            crate::signal_detector::types::TaxSignalRecord {
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
                            },
                        ));
                    }
                }
                _ => {
                    // Log other tax signals but don't convert to specific signal types yet
                    info!("💸 Tax signal detected: {:?}", tax_signal);
                }
            }
        }

        // STEP 2: Trading status detector (ACTIVE) - Now with tax values
        // Check for trading status changes or detect already-enabled trading
        if let Some(trading_signal) = self
            .trading_status_detector
            .detect(result, calculated_buy_tax, calculated_sell_tax)
            .await
        {
            // Always log what we detected
            let status_msg = match trading_signal.status_change {
                TradingStatusChange::TradingEnabled => {
                    // Check if this is a detection of already-enabled trading
                    if trading_signal.details.contains("ALREADY ENABLED") {
                        "Trading Detected (Already Enabled)"
                    } else {
                        "Trading Enabled"
                    }
                }
                TradingStatusChange::TradingDisabled => "Trading Disabled",
                TradingStatusChange::TradingPaused => "Trading Paused",
                TradingStatusChange::NoChange => "No Change",
            };

            self.log_activity(
                "TRADING_STATUS",
                &format!(
                    "Status: {} | Can Trade: {} | Details: {}",
                    status_msg, trading_signal.can_trade_after, trading_signal.details
                ),
            );

            match trading_signal.status_change {
                TradingStatusChange::TradingEnabled => {
                    // CRITICAL: Create pool-specific trading enabled signal
                    // Each pool gets its own signal with unique pool_address
                    let pool_address = result
                        .pool_address
                        .map(|addr| to_checksum_address(&addr))
                        .unwrap_or_else(|| "unknown".to_string());
                    let pool_type = result.pool_type.clone().unwrap_or_else(|| "V2".to_string());

                    signals.push(Signal::TradingEnabled(
                        crate::signal_detector::TradingEnabledSignal {
                            tx_hash: result.request.tx.hash.clone(),
                            token_address: trading_signal.token_address.clone(),
                            pool_address, // Now includes the specific pool
                            pool_type,    // Pool type (V2, V3, V4)
                            creator_address: trading_signal.executor.clone(),
                            buy_tax: calculated_buy_tax.unwrap_or(0.0),
                            sell_tax: calculated_sell_tax.unwrap_or(0.0),
                            timestamp: chrono::Utc::now().timestamp() as u64,
                        },
                    ));
                }
                _ => {}
            }
        } else {
            // No trading signal detected - log what we found
            // Skip trading status logging for contract creation transactions to reduce noise
            if !matches!(
                result.request.category,
                crate::tx_router::TransactionCategory::ContractCreation { .. }
            ) {
                if let Some(ref buy_sell) = result.buy_sell_result() {
                    let buy_tax = buy_sell
                        .buy_tax
                        .map(|v| format!("{:.1}%", v))
                        .unwrap_or_else(|| {
                            buy_sell
                                .buy_tax_error
                                .as_ref()
                                .map(|e| format!("ERR: {}", e))
                                .unwrap_or_else(|| "unknown".to_string())
                        });
                    let sell_tax = buy_sell
                        .sell_tax
                        .map(|v| format!("{:.1}%", v))
                        .unwrap_or_else(|| {
                            buy_sell
                                .sell_tax_error
                                .as_ref()
                                .map(|e| format!("ERR: {}", e))
                                .unwrap_or_else(|| "unknown".to_string())
                        });

                    self.log_activity(
                        "TRADING_STATUS",
                        &format!(
                            "No change | Can Buy: {} | Can Sell: {} | buy_tax: {} | sell_tax: {}",
                            buy_sell.can_buy, buy_sell.can_sell, buy_tax, sell_tax
                        ),
                    );
                } else {
                    self.log_activity("TRADING_STATUS", "No change (no buy/sell result)");
                }
            }
        }

        // STEP 3: Liquidity detector - Check for pool drains and liquidity removals
        // Use tx_state_changes which has the actual transaction state changes

        // Parse sender address
        if let Some(from_address) =
            alloy_primitives::Address::try_from(result.request.tx.from.as_slice()).ok()
        {
            // Prefer dedicated removal result if present
            let liquidity_signals = if let Some(ref removal_result) =
                result.liquidity_removal_result
            {
                info!(
                    "💧 Using dedicated liquidity removal detection for TX {}",
                    result.request.tx.hash
                );
                info!(
                    "  removal_result: success={} | pool_address={:?} | eth_removed={:.6} | drain%={:.2} | remaining_eth={:.6} | address_balance_changes_addrs={}",
                    removal_result.success,
                    removal_result.pool_address,
                    removal_result.eth_removed,
                    removal_result.drain_percentage,
                    removal_result.remaining_eth,
                    removal_result.address_balance_changes.len()
                );

                if !removal_result.success {
                    // Write to liquidity_removals.log file directly
                    let liquidity_log_path = self
                        .signal_log_path
                        .parent()
                        .unwrap_or(std::path::Path::new("."))
                        .join("liquidity_removals.log");
                    if let Ok(mut file) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&liquidity_log_path)
                    {
                        let timestamp = chrono::Local::now();
                        let token_str = result
                            .token_address
                            .map(|a| to_checksum_address(&a))
                            .unwrap_or_else(|| "Unknown".to_string());
                        let pool_str = result
                            .pool_address
                            .map(|a| to_checksum_address(&a))
                            .unwrap_or_else(|| "Unknown".to_string());
                        let remover_str = to_checksum_address(&from_address);
                        writeln!(file, "[{}] REMOVAL_FAILED | Pool: {} | Token: {} | Remover: {} | Reason: {} | TxHash: {}",
                            timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                            pool_str,
                            token_str,
                            remover_str,
                            removal_result.revert_reason.as_ref().unwrap_or(&"Unknown error".to_string()),
                            result.request.tx.hash
                        ).ok();
                        writeln!(file, "").ok();
                        file.flush().ok();
                    }
                    self.log_activity(
                        "LIQUIDITY_REMOVAL_FAILED",
                        &format!(
                            "TX: {} | Reason: {}",
                            result.request.tx.hash,
                            removal_result
                                .revert_reason
                                .as_ref()
                                .unwrap_or(&"Unknown error".to_string())
                        ),
                    );
                    error!(
                        "❌ LIQUIDITY_REMOVAL_FAILED | TX: {} | Reason: {}",
                        result.request.tx.hash,
                        removal_result
                            .revert_reason
                            .as_ref()
                            .unwrap_or(&"Unknown error".to_string())
                    );
                }

                if let Some(token_addr) = result.token_address {
                    if let Some(signal) = self
                        .liquidity_detector
                        .detect_from_removal_result(
                            &result.request.tx.hash,
                            from_address,
                            removal_result,
                            token_addr,
                        )
                        .await
                    {
                        vec![signal]
                    } else {
                        vec![]
                    }
                } else {
                    warn!("No token address available for liquidity removal detection");
                    vec![]
                }
            } else {
                if let Some(ref pool_result) = result.pool_viability_result {
                    self.liquidity_detector
                        .detect(
                            &result.request.tx.hash,
                            from_address,
                            &pool_result.buy_transaction.address_balance_changes,
                        )
                        .await
                } else {
                    vec![]
                }
            };

            // No fallback: rely strictly on the chosen detection path per TX

            // Emit unified LiquidityRemoval signals
            for liq_signal in liquidity_signals {
                let removal_signal = crate::signal_detector::LiquidityRemovalSignal {
                    tx_hash: liq_signal.tx_hash.clone(),
                    pool_address: liq_signal.pool_address.clone(),
                    pool_type: liq_signal.pool_type.clone(),
                    token_address: Some(liq_signal.token_address.clone()),
                    remover_address: liq_signal.from_address.clone(),
                    function_name: match liq_signal.change_type {
                        super::liquidity_detector::LiquidityChangeType::CompleteDrain => {
                            "liquidity_drain"
                        }
                        _ => "remove_liquidity",
                    }
                    .to_string(),
                    estimated_eth_removed: Some(liq_signal.eth_change.abs()),
                    timestamp: chrono::Utc::now().timestamp() as u64,
                };
                self.log_activity(
                    "LIQUIDITY_REMOVAL",
                    &format!(
                    "Pool: {} | Type: {} | ETH removed: {:.4} | Remaining: {:.4} | Drain %: {:.1}%",
                    liq_signal.pool_address,
                    liq_signal.pool_type,
                    liq_signal.eth_change.abs(),
                    liq_signal.remaining_liquidity,
                    liq_signal.percentage_change
                ),
                );
                info!("💧 Liquidity removal detected: {}", liq_signal.details);
                signals.push(Signal::LiquidityRemoval(removal_signal));
            }
        } else {
            // Log that we couldn't parse the from address
            self.log_activity(
                "LIQUIDITY_ERROR",
                "Invalid from address - skipping liquidity detection",
            );
        }

        // Log all detected signals
        for signal in &signals {
            self.log_signal(signal);
        }

        // STEP 4 (trimmed): omit detailed pool state change debug logging

        // Also log a summary for this simulation
        if !signals.is_empty() {
            debug!(
                "Detected {} signals for TX {}",
                signals.len(),
                result.request.tx.hash
            );
            self.total_signals_emitted += signals.len() as u64;
            self.log_activity(
                "SIGNALS_SUMMARY",
                &format!(
                    "Signals this TX: {} | Cumulative total: {}",
                    signals.len(),
                    self.total_signals_emitted
                ),
            );

            // Publish all detected signals immediately
            if let Some(ref publisher) = self.publisher {
                let mut pub_guard = publisher.lock().await;
                for signal in &signals {
                    if let Err(e) = pub_guard.publish(signal.clone()).await {
                        error!("Failed to publish signal: {}", e);
                    }
                }
                debug!("Published {} signals", signals.len());
            }
        }
        signals
    }

    /// Detect LP approval signals from non-simulated transactions
    pub async fn detect_lp_approval(
        &mut self,
        tx: &crate::mempool_fetcher::MempoolTransaction,
        category: &crate::tx_router::TransactionCategory,
    ) {
        // Check for LP approval
        if let Some(lp_signal) = self
            .lp_approval_detector
            .detect_from_transaction(tx, category)
        {
            // Publish the signal immediately if we have a publisher
            if let Some(ref publisher) = self.publisher {
                // Create an LP approval signal
                let mut enriched_signal = lp_signal.clone();

                // Compute approval percentage using cached pool data if available
                if let Some(ref token_cache) = self.token_cache {
                    if let Some(pool_state) = token_cache
                        .get_pool_by_address(&lp_signal.lp_token_address)
                        .await
                    {
                        if let Some(pct) = pool_state.lp_tokens_approved_percentage {
                            enriched_signal.approval_percentage = Some(pct.min(100.0));
                        }
                    }
                }

                // Fallback: treat max approval as 100%
                if enriched_signal.approval_percentage.is_none()
                    && enriched_signal.amount == U256::MAX
                {
                    enriched_signal.approval_percentage = Some(100.0);
                }

                let signal = Signal::LpApproval(enriched_signal.clone());

                // Log the signal
                self.log_signal(&signal);

                // Publish it
                let mut pub_guard = publisher.lock().await;
                if let Err(e) = pub_guard.publish(signal.clone()).await {
                    error!("Failed to publish LP approval signal: {}", e);
                } else {
                    info!(
                        "Successfully published LP approval signal for {}",
                        enriched_signal.tx_hash
                    );
                }
            }
        } else {
        }
    }
}
