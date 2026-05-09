/// Trading Status Detector
///
/// Detects when trading becomes enabled on pools based on simulation results.
/// Triggers signals when: can_buy && can_sell && taxes <= 25% && not already enabled
use crate::simulator::SimulationResult;
use crate::token_tracking::TokenTrackingCache;
use reth_chain_query::to_checksum_address;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct TradingStatusSignal {
    pub token_address: String,
    pub pool_address: String,
    pub pool_type: String, // Pool type (UniswapV2, UniswapV3, etc.)
    pub status_change: TradingStatusChange,
    pub executor: String,
    pub can_trade_after: bool,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub tx_hash: String,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TradingStatusChange {
    /// Trading has been enabled
    TradingEnabled,
    /// Trading has been disabled
    TradingDisabled,
    /// Trading was paused
    TradingPaused,
    /// Trading status unchanged
    NoChange,
}

pub struct TradingStatusDetector {
    /// Tax threshold for considering trading "enabled" (default: 25%)
    tax_threshold: f64,
    /// Path to the log file
    log_file_path: Option<PathBuf>,
    /// Token tracking cache to check existing trading status
    token_cache: Option<Arc<TokenTrackingCache>>,
    /// Tracks (token, pool) pairs we have already emitted signals for during this run
    emitted_trading_pairs: Arc<Mutex<HashSet<(String, String)>>>,
}

impl TradingStatusDetector {
    pub fn new() -> Self {
        Self {
            tax_threshold: 25.0, // 25% tax threshold
            log_file_path: None,
            token_cache: None,
            emitted_trading_pairs: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn with_log_path(log_path: PathBuf) -> Self {
        Self {
            tax_threshold: 25.0,
            log_file_path: Some(log_path),
            token_cache: None,
            emitted_trading_pairs: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Set the token cache for checking existing trading status
    pub fn set_token_cache(&mut self, cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(cache);
    }

    /// Detect trading enabled signals from simulation results
    /// Triggers when: can_buy && can_sell && taxes <= threshold && not already enabled
    /// Tax values are extracted from sim_result.buy_sell_result (calculated in simulation_manager)
    pub async fn detect(
        &self,
        sim_result: &SimulationResult,
        _buy_tax: Option<f64>,
        _sell_tax: Option<f64>,
    ) -> Option<TradingStatusSignal> {
        // Get buy/sell results from simulation
        let buy_sell = sim_result.buy_sell_result()?;

        // Extract transaction details
        let tx_hash = &sim_result.request.tx.hash;

        // Extract token/pool info from simulation result - MUST use the actual pool from simulation
        let token_address = if let Some(token_addr) = &sim_result.token_address {
            to_checksum_address(token_addr)
        } else {
            // Fallback to transaction category if not in sim result
            match &sim_result.request.category {
                crate::tx_router::TransactionCategory::CreatorTransaction {
                    target_token, ..
                } => target_token.as_ref()?.clone(),
                crate::tx_router::TransactionCategory::ContractCreation {
                    contract_address,
                    ..
                } => contract_address.clone(),
                _ => return None,
            }
        };

        // CRITICAL: Use pool address from simulation result, NOT from transaction category
        let pool_address = if let Some(pool_addr) = &sim_result.pool_address {
            to_checksum_address(pool_addr)
        } else {
            warn!(
                "No pool address in simulation result for token {}",
                token_address
            );
            return None;
        };

        // Get pool type from simulation result
        let pool_type = sim_result
            .pool_type
            .as_ref()
            .unwrap_or(&"Unknown".to_string())
            .clone();

        let already_enabled_in_cache = self
            .is_trading_already_enabled(&token_address, &pool_address)
            .await;
        if already_enabled_in_cache {
            debug!(
                "Pool {} for token {} already marked trading_enabled in cache",
                pool_address, token_address
            );
        }

        // Get executor from transaction category
        let executor = match &sim_result.request.category {
            crate::tx_router::TransactionCategory::CreatorTransaction { creator, .. } => {
                creator.clone()
            }
            crate::tx_router::TransactionCategory::ContractCreation { deployer, .. } => {
                deployer.clone()
            }
            _ => return None,
        };

        // Extract tax values from simulation result (calculated in simulation_manager)
        let buy_tax = buy_sell.buy_tax;
        let sell_tax = buy_sell.sell_tax;

        // Always log simulation results for debugging (now includes tax values)
        self.log_simulation_result(
            &token_address,
            &pool_address,
            &buy_sell,
            tx_hash,
            buy_tax,
            sell_tax,
        );

        // Check if trading works (both buy and sell)
        if !buy_sell.can_buy || !buy_sell.can_approve || !buy_sell.can_sell {
            info!(
                "⚠️ TradingEnabled skipped for token {} pool {} (tx {}): can_buy={} can_approve={} can_sell={}",
                token_address, pool_address, tx_hash, buy_sell.can_buy, buy_sell.can_approve, buy_sell.can_sell
            );
            return None;
        }

        // Check if taxes are reasonable (below threshold)
        // If taxes are too high, trading is not really "enabled" in a practical sense
        // IMPORTANT: If tax calculation fails (None), we cannot generate a TRADING_ENABLED signal
        match buy_tax {
            Some(buy_t) if buy_t > self.tax_threshold => {
                info!(
                    "⚠️ TradingEnabled skipped for token {} pool {} (tx {}): buy tax {:.1}% exceeds threshold {:.1}%",
                    token_address, pool_address, tx_hash, buy_t, self.tax_threshold
                );
                return None;
            }
            None => {
                // Show the specific error if available
                let error_detail = buy_sell
                    .buy_tax_error
                    .as_ref()
                    .map(|e| format!(": {}", e))
                    .unwrap_or_default();
                warn!(
                    "INVALID_SIMULATION_RESULTS: Token {} pool {} (tx {}) - Cannot generate TRADING_ENABLED signal: buy tax calculation failed{}",
                    token_address, pool_address, tx_hash, error_detail
                );
                return None;
            }
            Some(buy_t) => {
                debug!(
                    "Token {} pool {} - Buy tax acceptable: {:.1}%",
                    token_address, pool_address, buy_t
                );
            }
        }

        match sell_tax {
            Some(sell_t) if sell_t > self.tax_threshold => {
                info!(
                    "⚠️ TradingEnabled skipped for token {} pool {} (tx {}): sell tax {:.1}% exceeds threshold {:.1}%",
                    token_address, pool_address, tx_hash, sell_t, self.tax_threshold
                );
                return None;
            }
            None => {
                // Show the specific error if available
                let error_detail = buy_sell
                    .sell_tax_error
                    .as_ref()
                    .map(|e| format!(": {}", e))
                    .unwrap_or_default();
                warn!(
                    "INVALID_SIMULATION_RESULTS: Token {} pool {} (tx {}) - Cannot generate TRADING_ENABLED signal: sell tax calculation failed{}",
                    token_address, pool_address, tx_hash, error_detail
                );
                return None;
            }
            Some(sell_t) => {
                debug!(
                    "Token {} pool {} - Sell tax acceptable: {:.1}%",
                    token_address, pool_address, sell_t
                );
            }
        }

        // Prevent duplicate TradingEnabled signals during this run
        let pair_key = (token_address.clone(), pool_address.clone());
        {
            let mut emitted = self.emitted_trading_pairs.lock().await;
            if emitted.contains(&pair_key) {
                info!(
                    "ℹ️ TradingEnabled dedupe: token {} pool {} (tx {}) already emitted earlier in this run",
                    token_address, pool_address, tx_hash
                );
                return None;
            }
            emitted.insert(pair_key);
        }

        // All conditions met - generate trading enabled signal!
        info!(
            "🚀 TRADING ENABLED SIGNAL: Token {} Pool {} | Buy: {:.1}% | Sell: {:.1}% | TX: {}",
            token_address,
            pool_address,
            buy_tax.unwrap_or(0.0),
            sell_tax.unwrap_or(0.0),
            tx_hash
        );

        Some(TradingStatusSignal {
            token_address: token_address.to_string(),
            pool_address: pool_address.to_string(),
            pool_type: pool_type.clone(),
            status_change: TradingStatusChange::TradingEnabled,
            executor,
            can_trade_after: true,
            buy_tax,
            sell_tax,
            tx_hash: tx_hash.clone(),
            details: format!(
                "Trading enabled: buy_tax={:.1}%, sell_tax={:.1}%, cache_already_enabled={}",
                buy_tax.unwrap_or(0.0),
                sell_tax.unwrap_or(0.0),
                already_enabled_in_cache
            ),
        })
    }

    /// Log simulation results to file for debugging (always log everything)
    fn log_simulation_result(
        &self,
        token_address: &str,
        pool_address: &str,
        buy_sell: &crate::simulator::BuySellResult,
        tx_hash: &str,
        buy_tax: Option<f64>,
        sell_tax: Option<f64>,
    ) {
        if let Some(ref log_path) = self.log_file_path {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
                let timestamp = chrono::Local::now();

                // Format buy tax: show percentage if calculated, or error if failed
                let buy_tax_str = match buy_tax {
                    Some(tax) => format!("{:.1}%", tax),
                    None => match &buy_sell.buy_tax_error {
                        Some(error) => format!("ERROR: {}", error),
                        None => "ERROR: Unknown".to_string(),
                    },
                };

                // Format sell tax: show percentage if calculated, or error if failed
                let sell_tax_str = match sell_tax {
                    Some(tax) => format!("{:.1}%", tax),
                    None => match &buy_sell.sell_tax_error {
                        Some(error) => format!("ERROR: {}", error),
                        None => "ERROR: Unknown".to_string(),
                    },
                };

                writeln!(
                    file,
                    "[{}] SIMULATION_RESULT | TX: {} | Token: {} | Pool: {} | can_buy: {} | can_approve: {} | can_sell: {} | buy_tax: {} | sell_tax: {}",
                    timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                    tx_hash,
                    token_address,
                    pool_address,
                    buy_sell.can_buy,
                    buy_sell.can_approve,
                    buy_sell.can_sell,
                    buy_tax_str,
                    sell_tax_str
                )
                .ok();
                writeln!(file, "").ok(); // Add empty line for readability
            }
        }
    }

    /// Check if trading is already enabled for this token/pool combination
    async fn is_trading_already_enabled(&self, token_address: &str, pool_address: &str) -> bool {
        if let Some(ref cache) = self.token_cache {
            // Get all pools for this token
            let pools = cache.get_pools_for_token(&token_address.to_string()).await;

            // Find the specific pool
            if let Some(pool_state) = pools.iter().find(|p| p.address == pool_address) {
                // Check the actual trading_enabled flag for this pool
                if pool_state.trading_enabled {
                    debug!(
                        "Pool {} already has trading_enabled=true in cache",
                        pool_address
                    );
                    return true;
                }
            }
        }
        false
    }
}
