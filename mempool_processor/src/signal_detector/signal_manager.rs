use crate::config::TaxDetectionConfig;
use crate::position_approval_call::decode_position_approval_call;
use crate::signal_publisher::SignalPublisher;
use crate::simulator::{BuySellResult, SimulationResult};
use crate::token_tracking::TokenTrackingCache;
use reth_chain_query::to_checksum_address;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use super::{
    build_position_approval_signals, enrich_erc20_liquidity_approval,
    trading_status_detector::TradingStatusChange, LiquidityDetector, LpApprovalDetector, Signal,
    TaxDetector, TaxSignalType, TokenSupplyRiskDetector, TradingStatusDetector,
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
    token_supply_risk_detector: TokenSupplyRiskDetector,
    lp_approval_detector: LpApprovalDetector,
    token_cache: Option<Arc<TokenTrackingCache>>,
    signal_log_path: PathBuf,
    error_log_path: PathBuf,
    publisher: Option<Arc<Mutex<SignalPublisher>>>,
    total_signals_emitted: u64,
    emitted_honeypot_pairs: HashSet<(String, String)>,
    emitted_tax_states: HashSet<(String, String, String)>,
}

impl SignalManager {
    /// Create a new signal manager
    pub fn new(config: SignalManagerConfig) -> Self {
        info!("🔍 Signal manager initialized");
        info!("📁 Using log directory: {}", config.log_dir.display());

        // Create log directory if it doesn't exist
        std::fs::create_dir_all(&config.log_dir).ok();

        // Create detector-specific log files. The signals directory is for
        // semantic signal logs only; generic simulation diagnostics go to the
        // run-level simulation_errors.log file.
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
            trading_status_detector: TradingStatusDetector::new(),
            tax_signal_detector: TaxDetector::new(config.tax_detection.clone()),
            token_supply_risk_detector: TokenSupplyRiskDetector::new(),
            lp_approval_detector: LpApprovalDetector::new(&config.log_dir),
            token_cache: None,
            signal_log_path,
            error_log_path,
            publisher: None,
            total_signals_emitted: 0,
            emitted_honeypot_pairs: HashSet::new(),
            emitted_tax_states: HashSet::new(),
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

    fn log_buy_sell_simulation_errors(&self, result: &SimulationResult, buy_sell: &BuySellResult) {
        if result
            .pool_viability_result
            .as_ref()
            .and_then(|pool_result| pool_result.failure_reason.as_deref())
            .map(is_replay_context_mismatch)
            .unwrap_or(false)
        {
            return;
        }

        if let Some(ref error) = buy_sell.buy_tax_error {
            self.log_error(
                "BUY_TAX_ERROR",
                &format_buy_sell_error(result, error.as_str()),
            );
        }
        if let Some(ref error) = buy_sell.sell_tax_error {
            self.log_error(
                "SELL_TAX_ERROR",
                &format_buy_sell_error(result, error.as_str()),
            );
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

                    format!(
                        "[{}] SIGNAL_DETECTED | TAX_SIGNAL | {} | token: {} | pool: {} | buy_tax: {} | sell_tax: {} | type: {}",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        s.tx_hash,
                        s.token_address,
                        s.pool_address,
                        buy_tax_str,
                        sell_tax_str,
                        s.signal_type
                    )
                }
                Signal::Honeypot(s) => {
                    format!(
                        "[{}] SIGNAL_DETECTED | SELL_BLOCKED | {} | token: {} | pool: {} | pool_type: {} | creator: {} | can_buy: {} | can_sell: {}",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        s.tx_hash,
                        s.token_address,
                        s.pool_address,
                        s.pool_type,
                        s.creator_address,
                        s.can_buy,
                        s.can_sell
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
                Signal::TokenSupplyRisk(s) => {
                    format!(
                        "[{}] SIGNAL_DETECTED | TOKEN_SUPPLY_RISK | {} | token: {} | risk_type: {} | actor: {} | block: {}",
                        timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                        s.tx_hash.as_deref().unwrap_or("none"),
                        s.token_address,
                        s.risk_type,
                        s.actor_address.as_deref().unwrap_or("unknown"),
                        s.block_number
                            .map(|block| block.to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    )
                }
            };

            writeln!(file, "{}", log_entry).ok();
        }
    }

    /// Process one pool-specific simulation result into publishable signals.
    pub async fn process_simulation_result(&mut self, result: &SimulationResult) -> Vec<Signal> {
        let mut signals = Vec::new();
        let result_pool_address = result.pool_address.map(|addr| to_checksum_address(&addr));
        let pool_context =
            pool_context_for_address(&self.token_cache, result_pool_address.as_deref()).await;

        let buy_sell_result = result.buy_sell_result();
        let (_buy_tax, _sell_tax, _can_buy, _can_sell) = if let Some(buy_sell) = &buy_sell_result {
            self.log_buy_sell_simulation_errors(result, buy_sell);
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

        if let Some(buy_sell) = &buy_sell_result {
            if self.tax_signal_detector.is_honeypot(
                buy_sell.buy_tax,
                buy_sell.sell_tax,
                buy_sell.can_buy,
                buy_sell.can_approve,
                buy_sell.can_sell,
            ) {
                if let (Some(token_address), Some(pool_address)) = (
                    token_address_from_simulation_result(result),
                    result_pool_address.clone(),
                ) {
                    if self
                        .emitted_honeypot_pairs
                        .insert((token_address.clone(), pool_address.clone()))
                    {
                        let pool_type =
                            result.pool_type.clone().unwrap_or_else(|| "V2".to_string());
                        let creator_address = creator_address_from_simulation_result(result);
                        let failure_reason = result
                            .pool_viability_result
                            .as_ref()
                            .and_then(|pool_result| pool_result.failure_reason.clone());

                        signals.push(Signal::Honeypot(
                            crate::signal_detector::types::HoneypotSignal {
                                tx_hash: result.request.tx.hash.clone(),
                                token_address,
                                pool_address,
                                pool_type,
                                denom_address: pool_context
                                    .as_ref()
                                    .map(|context| context.denom_address.clone()),
                                denom_currency: pool_context
                                    .as_ref()
                                    .map(|context| context.denom_currency.clone()),
                                denom_decimals: pool_context
                                    .as_ref()
                                    .and_then(|context| context.denom_decimals),
                                creator_address,
                                can_buy: buy_sell.can_buy,
                                can_sell: buy_sell.can_sell,
                                buy_tax: buy_sell.buy_tax,
                                sell_tax: buy_sell.sell_tax,
                                failure_reason,
                                confidence: 0.95,
                                timestamp: chrono::Utc::now().timestamp() as u64,
                            },
                        ));
                    }
                }
            }
        }

        let tax_signals = self.tax_signal_detector.detect(result);

        let (calculated_buy_tax, calculated_sell_tax) =
            if let Some(ref buy_sell) = result.buy_sell_result() {
                (buy_sell.buy_tax, buy_sell.sell_tax)
            } else {
                (None, None)
            };

        for tax_signal in &tax_signals {
            let (signal_type, buy_tax_exceeds_threshold, sell_tax_exceeds_threshold) =
                match tax_signal.signal_type {
                    TaxSignalType::TaxBucketRisk {
                        buy_tax_exceeds_threshold,
                        sell_tax_exceeds_threshold,
                    } => (
                        "TaxBucketRisk",
                        buy_tax_exceeds_threshold,
                        sell_tax_exceeds_threshold,
                    ),
                    TaxSignalType::TaxChange { .. } => ("TaxChange", false, false),
                    TaxSignalType::SuspiciousPattern => ("SuspiciousPattern", false, false),
                };

            let should_log_tax_signal = if let Some(ref token_cache) = self.token_cache {
                let pools = token_cache
                    .get_pools_for_token(&tax_signal.token_address)
                    .await;
                let trading_enabled = pools.iter().any(|p| p.trading_enabled);

                if trading_enabled {
                    true
                } else if let Some(ref bs) = buy_sell_result {
                    bs.can_buy || bs.can_sell
                } else {
                    false
                }
            } else {
                true
            };

            if should_log_tax_signal {
                let pool_address = result_pool_address
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());
                let pool_type = result.pool_type.clone().unwrap_or_else(|| "V2".to_string());
                let creator_address = creator_address_from_simulation_result(result);
                let state_key = format!(
                    "{}:{}:{}:{}:{}:{}",
                    signal_type,
                    tax_signal.buy_tax_bucket_from.as_deref().unwrap_or("none"),
                    tax_signal.buy_tax_bucket_to.as_deref().unwrap_or("none"),
                    tax_signal.sell_tax_bucket_from.as_deref().unwrap_or("none"),
                    tax_signal.sell_tax_bucket_to.as_deref().unwrap_or("none"),
                    tax_signal
                        .combined_tax_bucket_to
                        .as_deref()
                        .unwrap_or("none")
                );

                if !self.emitted_tax_states.insert((
                    tax_signal.token_address.clone(),
                    pool_address.clone(),
                    state_key,
                )) {
                    continue;
                }

                signals.push(Signal::TaxSignal(
                    crate::signal_detector::types::TaxSignalRecord {
                        tx_hash: result.request.tx.hash.clone(),
                        token_address: tax_signal.token_address.clone(),
                        pool_address,
                        pool_type,
                        denom_address: pool_context
                            .as_ref()
                            .map(|context| context.denom_address.clone()),
                        denom_currency: pool_context
                            .as_ref()
                            .map(|context| context.denom_currency.clone()),
                        denom_decimals: pool_context
                            .as_ref()
                            .and_then(|context| context.denom_decimals),
                        creator_address,
                        signal_type: signal_type.to_string(),
                        signal_details: tax_signal.details.clone(),
                        confidence: tax_signal.confidence,
                        buy_tax: tax_signal.buy_tax,
                        sell_tax: tax_signal.sell_tax,
                        buy_tax_bucket_from: tax_signal.buy_tax_bucket_from.clone(),
                        buy_tax_bucket_to: tax_signal.buy_tax_bucket_to.clone(),
                        sell_tax_bucket_from: tax_signal.sell_tax_bucket_from.clone(),
                        sell_tax_bucket_to: tax_signal.sell_tax_bucket_to.clone(),
                        combined_tax_bucket_from: tax_signal.combined_tax_bucket_from.clone(),
                        combined_tax_bucket_to: tax_signal.combined_tax_bucket_to.clone(),
                        buy_tax_exceeds_threshold,
                        sell_tax_exceeds_threshold,
                        cant_sell: false,
                        timestamp: chrono::Utc::now().timestamp() as u64,
                    },
                ));
            }
        }

        if let Some(signal) = self.token_supply_risk_detector.detect(result) {
            signals.push(signal);
        }

        if let Some(trading_signal) = self
            .trading_status_detector
            .detect(result, calculated_buy_tax, calculated_sell_tax)
            .await
        {
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
                    let pool_address = result_pool_address
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string());
                    let pool_type = result.pool_type.clone().unwrap_or_else(|| "V2".to_string());

                    signals.push(Signal::TradingEnabled(
                        crate::signal_detector::TradingEnabledSignal {
                            tx_hash: result.request.tx.hash.clone(),
                            token_address: trading_signal.token_address.clone(),
                            pool_address, // Now includes the specific pool
                            pool_type,    // Pool type (V2, V3, V4)
                            denom_address: pool_context
                                .as_ref()
                                .map(|context| context.denom_address.clone()),
                            denom_currency: pool_context
                                .as_ref()
                                .map(|context| context.denom_currency.clone()),
                            denom_decimals: pool_context
                                .as_ref()
                                .and_then(|context| context.denom_decimals),
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
        }

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
                let liquidity_pool_context =
                    pool_context_for_address(&self.token_cache, Some(&liq_signal.pool_address))
                        .await
                        .or_else(|| pool_context.clone());
                let removal_signal = crate::signal_detector::LiquidityRemovalSignal {
                    tx_hash: liq_signal.tx_hash.clone(),
                    pool_address: liq_signal.pool_address.clone(),
                    pool_type: liq_signal.pool_type.clone(),
                    denom_address: liquidity_pool_context
                        .as_ref()
                        .map(|context| context.denom_address.clone()),
                    denom_currency: liquidity_pool_context
                        .as_ref()
                        .map(|context| context.denom_currency.clone()),
                    denom_decimals: liquidity_pool_context
                        .as_ref()
                        .and_then(|context| context.denom_decimals),
                    token_address: Some(liq_signal.token_address.clone()),
                    remover_address: liq_signal.from_address.clone(),
                    function_name: liq_signal.function_name.clone().unwrap_or_else(|| {
                        match liq_signal.change_type {
                            super::liquidity_detector::LiquidityChangeType::CompleteDrain => {
                                "liquidity_drain"
                            }
                            _ => "remove_liquidity",
                        }
                        .to_string()
                    }),
                    estimated_eth_removed: liq_signal.eth_removed.or_else(|| {
                        if liq_signal.eth_change != 0.0 {
                            Some(liq_signal.eth_change.abs())
                        } else {
                            None
                        }
                    }),
                    remaining_eth: liq_signal.remaining_eth.or_else(|| {
                        if liq_signal.remaining_liquidity > 0.0 {
                            Some(liq_signal.remaining_liquidity)
                        } else {
                            None
                        }
                    }),
                    removal_percentage: liq_signal.removal_percentage.or_else(|| {
                        if liq_signal.percentage_change > 0.0 {
                            Some(liq_signal.percentage_change)
                        } else {
                            None
                        }
                    }),
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
    ) -> bool {
        if let (Some(token_cache), Some(position_approval)) =
            (self.token_cache.as_ref(), decode_position_approval_call(tx))
        {
            let position_manager = to_checksum_address(&position_approval.position_manager());
            if token_cache.is_position_manager(&position_manager).await {
                let signals =
                    build_position_approval_signals(token_cache, tx, position_approval).await;
                if signals.is_empty() {
                    debug!(
                        "Position approval {} passed routing but no mapped tracked position/share was found",
                        tx.hash
                    );
                    return false;
                }
                let Some(ref publisher) = self.publisher else {
                    return false;
                };
                let mut published = 0usize;
                let mut pub_guard = publisher.lock().await;
                for signal in signals {
                    if let Err(e) = pub_guard.publish(signal).await {
                        error!("Failed to publish position approval signal: {}", e);
                    } else {
                        published += 1;
                    }
                }
                return published > 0;
            }
        }

        // Check for LP approval
        if let Some(lp_signal) = self
            .lp_approval_detector
            .detect_from_transaction(tx, category)
        {
            // Publish the signal immediately if we have a publisher
            if let Some(ref publisher) = self.publisher {
                // LP approvals are actionable only when the LP token is a tracked pool.
                // Refuse un-enriched signals to keep regular ERC20 approvals out of
                // the liquidity-removal early-warning stream.
                let Some(ref token_cache) = self.token_cache else {
                    warn!(
                        "LP approval {} cannot be published without TokenTrackingCache enrichment",
                        lp_signal.tx_hash
                    );
                    return false;
                };
                let Some(pool_state) = token_cache
                    .get_pool_by_liquidity_ownership_token(&lp_signal.lp_token_address)
                    .await
                else {
                    warn!(
                        "LP approval {} passed routing but ownership token {} was not found in cache during enrichment",
                        lp_signal.tx_hash, lp_signal.lp_token_address
                    );
                    return false;
                };

                let enriched_signal =
                    enrich_erc20_liquidity_approval(lp_signal.clone(), &pool_state);
                if enriched_signal.approved_share_pct.is_none() {
                    warn!(
                        "LP approval {} is mapped to pool {} but cannot compute approved share; publishing unquantified pool-risk signal",
                        lp_signal.tx_hash, enriched_signal.pool_address
                    );
                }

                let signal = Signal::LpApproval(enriched_signal.clone());

                // Log the signal
                self.log_signal(&signal);

                // Publish it
                let mut pub_guard = publisher.lock().await;
                if let Err(e) = pub_guard.publish(signal.clone()).await {
                    error!("Failed to publish LP approval signal: {}", e);
                    return false;
                } else {
                    info!(
                        "Successfully published LP approval signal for {}",
                        enriched_signal.tx_hash
                    );
                    return true;
                }
            }
        }
        false
    }
}

fn token_address_from_simulation_result(result: &SimulationResult) -> Option<String> {
    if let Some(token_address) = &result.token_address {
        return Some(to_checksum_address(token_address));
    }

    match &result.request.category {
        crate::tx_router::TransactionCategory::CreatorTransaction { target_token, .. } => {
            target_token.clone()
        }
        crate::tx_router::TransactionCategory::ContractCreation {
            contract_address, ..
        } => Some(contract_address.clone()),
        _ => None,
    }
}

#[derive(Clone)]
struct SignalPoolContext {
    denom_address: String,
    denom_currency: String,
    denom_decimals: Option<u8>,
}

async fn pool_context_for_address(
    token_cache: &Option<Arc<TokenTrackingCache>>,
    pool_identifier: Option<&str>,
) -> Option<SignalPoolContext> {
    let cache = token_cache.as_ref()?;
    let pool_identifier = pool_identifier?;
    let pool = cache.get_pool_by_address(pool_identifier).await?;
    Some(SignalPoolContext {
        denom_address: pool.denom_address.clone(),
        denom_currency: pool.denom_currency.clone(),
        denom_decimals: known_denom_decimals(&pool.denom_currency, &pool.denom_address),
    })
}

fn known_denom_decimals(symbol: &str, address: &str) -> Option<u8> {
    match symbol.trim().to_ascii_uppercase().as_str() {
        "ETH" | "WETH" | "DAI" | "USDE" => Some(18),
        "USDC" | "USDT" | "EUROC" | "EURC" => Some(6),
        _ if address.eq_ignore_ascii_case("0x0000000000000000000000000000000000000000") => Some(18),
        _ if address.eq_ignore_ascii_case("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2") => Some(18),
        _ if address.eq_ignore_ascii_case("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48") => Some(6),
        _ => None,
    }
}

fn is_replay_context_mismatch(error: &str) -> bool {
    error.contains("Setup transaction replay failed") && error.contains("mined receipt succeeded")
}

fn creator_address_from_simulation_result(result: &SimulationResult) -> String {
    match &result.request.category {
        crate::tx_router::TransactionCategory::CreatorTransaction { creator, .. } => {
            creator.clone()
        }
        _ => "unknown".to_string(),
    }
}

fn format_buy_sell_error(result: &SimulationResult, error: &str) -> String {
    format!(
        "tx={} | token={} | pool={} | pool_type={} | can_buy={} | can_approve={} | can_sell={} | error={}",
        result.request.tx.hash,
        token_address_from_simulation_result(result).unwrap_or_else(|| "unknown".to_string()),
        result
            .pool_address
            .map(|addr| to_checksum_address(&addr))
            .unwrap_or_else(|| "unknown".to_string()),
        result.pool_type.as_deref().unwrap_or("unknown"),
        result
            .buy_sell_result()
            .map(|buy_sell| buy_sell.can_buy)
            .unwrap_or(false),
        result
            .buy_sell_result()
            .map(|buy_sell| buy_sell.can_approve)
            .unwrap_or(false),
        result
            .buy_sell_result()
            .map(|buy_sell| buy_sell.can_sell)
            .unwrap_or(false),
        error
    )
}
