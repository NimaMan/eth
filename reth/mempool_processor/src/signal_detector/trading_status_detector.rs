/// Trading Status Detector
/// 
/// Detects when trading becomes enabled on pools based on simulation results.
/// Triggers signals when: can_buy && can_sell && taxes <= 25% && not already enabled

use crate::simulator::SimulationResult;
use crate::token_tracking::TokenTrackingCache;
use tracing::{info, debug};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TradingStatusSignal {
    pub token_address: String,
    pub pool_address: String,
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
    /// Minimum ETH liquidity to consider trading already enabled
    min_liquidity_threshold: f64,
    /// Path to the log file
    log_file_path: Option<PathBuf>,
    /// Token tracking cache to check existing trading status
    token_cache: Option<Arc<TokenTrackingCache>>,
}

impl TradingStatusDetector {
    pub fn new() -> Self {
        Self {
            tax_threshold: 25.0, // 25% tax threshold
            min_liquidity_threshold: 0.5, // 0.5 ETH minimum liquidity
            log_file_path: None,
            token_cache: None,
        }
    }
    
    pub fn with_log_path(log_path: PathBuf) -> Self {
        Self {
            tax_threshold: 25.0,
            min_liquidity_threshold: 0.5, // 0.5 ETH minimum liquidity
            log_file_path: Some(log_path),
            token_cache: None,
        }
    }
    
    pub fn with_config(log_path: PathBuf, min_liquidity_threshold: f64) -> Self {
        Self {
            tax_threshold: 25.0,
            min_liquidity_threshold,
            log_file_path: Some(log_path),
            token_cache: None,
        }
    }
    
    /// Set the token cache for checking existing trading status
    pub fn set_token_cache(&mut self, cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(cache);
    }

    /// Detect trading enabled signals from simulation results
    /// Triggers when: can_buy && can_sell && taxes <= threshold && not already enabled
    /// Tax values should be provided from tax_detector, not calculated here
    pub async fn detect(&self, sim_result: &SimulationResult, buy_tax: Option<f64>, sell_tax: Option<f64>) -> Option<TradingStatusSignal> {
        // Get buy/sell results from simulation
        let buy_sell = sim_result.buy_sell_result.as_ref()?;
        
        // Extract transaction details
        let tx_hash = &sim_result.request.tx.hash;
        
        // Extract token/pool info from transaction category
        let (token_address, pool_address, executor) = match &sim_result.request.category {
            crate::tx_router::TransactionCategory::CreatorTransaction { 
                creator, 
                target_token,
                target_address,
                .. 
            } => {
                let token = target_token.as_ref()?;
                // Use target_address as pool address
                (token.clone(), target_address.clone(), creator.clone())
            }
            crate::tx_router::TransactionCategory::ContractCreation { contract_address, deployer, .. } => {
                // For contract creation, the contract itself might be the token
                (contract_address.clone(), "unknown".to_string(), deployer.clone())
            }
            _ => return None,
        };

        // Always log simulation results for debugging (now includes tax values)
        self.log_simulation_result(&token_address, &pool_address, buy_sell, tx_hash, buy_tax, sell_tax);
        
        // Check if trading works (both buy and sell)
        if !buy_sell.can_buy || !buy_sell.can_sell {
            debug!("Token {} pool {} - Trading not working: can_buy={}, can_sell={}", 
                   token_address, pool_address, buy_sell.can_buy, buy_sell.can_sell);
            return None;
        }

        // Check if taxes are reasonable (below threshold)
        // If taxes are too high, trading is not really "enabled" in a practical sense
        if let Some(buy_t) = buy_tax {
            if buy_t > self.tax_threshold {
                debug!("Token {} pool {} - Buy tax too high: {:.1}%", token_address, pool_address, buy_t);
                return None;
            }
        }
        
        if let Some(sell_t) = sell_tax {
            if sell_t > self.tax_threshold {
                debug!("Token {} pool {} - Sell tax too high: {:.1}%", token_address, pool_address, sell_t);
                return None;
            }
        }
        
        // Check if trading is already enabled for this pool
        if self.is_trading_already_enabled(&token_address, &pool_address).await {
            debug!("Token {} pool {} - Trading already enabled, skipping signal", 
                   token_address, pool_address);
            return None;
        }
        
        // All conditions met - generate trading enabled signal!
        info!("🚀 TRADING ENABLED SIGNAL: Token {} Pool {} | Buy: {:.1}% | Sell: {:.1}% | TX: {}", 
              token_address, pool_address,
              buy_tax.unwrap_or(0.0), sell_tax.unwrap_or(0.0), tx_hash);
        
        Some(TradingStatusSignal {
            token_address: token_address.to_string(),
            pool_address: pool_address.to_string(),
            status_change: TradingStatusChange::TradingEnabled,
            executor,
            can_trade_after: true,
            buy_tax,
            sell_tax,
            tx_hash: tx_hash.clone(),
            details: format!("Trading enabled: buy_tax={:.1}%, sell_tax={:.1}%", 
                           buy_tax.unwrap_or(0.0), sell_tax.unwrap_or(0.0)),
        })
    }
    
    /// Log simulation results to file for debugging (always log everything)
    fn log_simulation_result(&self, token_address: &str, pool_address: &str, buy_sell: &crate::simulator::BuySellResult, tx_hash: &str, buy_tax: Option<f64>, sell_tax: Option<f64>) {
        if let Some(ref log_path) = self.log_file_path {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
            {
                let timestamp = chrono::Local::now();
                writeln!(file, 
                    "[{}] SIMULATION_RESULT | TX: {} | Token: {} | Pool: {} | can_buy: {} | can_sell: {} | buy_tax: {:.1}% | sell_tax: {:.1}%",
                    timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                    tx_hash,
                    token_address,
                    pool_address,
                    buy_sell.can_buy,
                    buy_sell.can_sell,
                    buy_tax.unwrap_or(-1.0),  // -1 indicates not calculated
                    sell_tax.unwrap_or(-1.0)   // -1 indicates not calculated
                ).ok();
                writeln!(file, "").ok(); // Add empty line for readability
            }
        }
    }
    
    /// Check if trading is already enabled for this token/pool combination
    async fn is_trading_already_enabled(&self, token_address: &str, pool_address: &str) -> bool {
        if let Some(ref cache) = self.token_cache {
            // Get pool state from cache
            if let Some((_, pool_state)) = cache.get_pools_for_token(token_address).await.into_iter()
                .find(|(addr, _)| addr == pool_address) 
            {
                // Check if this pool has significant liquidity (indicates trading is working)
                if pool_state.eth_reserve > self.min_liquidity_threshold {
                    return true;
                }
            }
        }
        false
    }
}