/// Tax Detector
/// 
/// Analyzes state changes to calculate taxes and detect tax-related signals.
/// This detector centralizes all tax calculation and detection logic.
///
/// Detects:
/// - High buy/sell taxes
/// - Honeypot patterns (extremely high sell tax)
/// - Tax changes (when compared to previous state)
/// - Suspicious tax patterns

use crate::simulator::SimulationResult;
use crate::token_tracking::{calculate_buy_tax, calculate_sell_tax};
use crate::config::TaxDetectionConfig;
use tracing::{info, debug};
use alloy_primitives::Address;
use hex;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use chrono;

#[derive(Debug, Clone)]
pub struct TaxSignal {
    pub token_address: String,
    pub signal_type: TaxSignalType,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub details: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaxSignalType {
    /// High tax or honeypot detected - consolidated signal
    /// Covers: high buy tax, high sell tax, honeypot (can't sell)
    /// Per-pool and per-token detection
    HighTaxOrHoneypot { 
        buy_tax_exceeds_threshold: bool, 
        sell_tax_exceeds_threshold: bool, 
        cant_sell: bool 
    },
    /// Tax change detected (requires historical data)
    TaxChange { before_buy: f64, before_sell: f64, after_buy: f64, after_sell: f64 },
    /// Suspicious tax pattern (e.g., low buy, high sell)
    SuspiciousPattern,
}

pub struct TaxDetector {
    /// Tax detection configuration
    config: TaxDetectionConfig,
    /// Path to the log file
    log_file_path: Option<PathBuf>,
}

impl TaxDetector {
    pub fn new(config: TaxDetectionConfig) -> Self {
        Self {
            config,
            log_file_path: None,
        }
    }
    
    pub fn with_log_path(config: TaxDetectionConfig, log_path: PathBuf) -> Self {
        Self {
            config,
            log_file_path: Some(log_path),
        }
    }

    /// Detect tax-related signals from simulation result
    pub fn detect(&self, sim_result: &SimulationResult) -> Vec<TaxSignal> {
        let mut signals = Vec::new();

        // Skip if no buy/sell result
        let buy_sell = match &sim_result.buy_sell_result {
            Some(bs) => bs,
            None => return signals,
        };

        // Extract token and pool addresses
        let (token_address, pool_address) = match &sim_result.request.category {
            crate::tx_router::TransactionCategory::CreatorTransaction { target_token, .. } => {
                if let Some(token) = target_token {
                    // Need to find pool address - for now we'll use the one from simulation
                    (token.clone(), None::<Address>)
                } else {
                    return signals;
                }
            }
            crate::tx_router::TransactionCategory::ContractCreation { contract_address, .. } => {
                (contract_address.clone(), None)
            }
            _ => return signals,
        };

        // Calculate taxes from raw state changes
        let (calculated_buy_tax, calculated_sell_tax) = if let Some(buy_sell) = &sim_result.buy_sell_result {
            // Get buyer address from simulator (hardcoded for now)
            let buyer_address = alloy_primitives::Address::from_slice(&hex::decode("70997970C51812dc3A010C7d01b50e0d17dc79C8").unwrap_or_default());
            
            let mut buy_tax = None;
            let mut sell_tax = None;
            
            // Calculate buy tax if we have the state changes and addresses
            if let (Some(buy_changes), Some(token_addr), Some(pool_addr)) = (&buy_sell.buy_state_changes, &sim_result.token_address, &sim_result.pool_address) {
                buy_tax = calculate_buy_tax(buy_changes, pool_addr, &buyer_address, token_addr);
                if buy_tax.is_none() {
                    debug!("Buy tax calculation returned None for token {:?} pool {:?}", token_addr, pool_addr);
                }
            } else {
                info!("Missing data for buy tax calculation - buy_changes: {}, token_addr: {}, pool_addr: {}", 
                    buy_sell.buy_state_changes.is_some(), 
                    sim_result.token_address.is_some(), 
                    sim_result.pool_address.is_some());
            }
            
            // Calculate sell tax if we have the state changes and addresses
            if let (Some(sell_changes), Some(pool_addr)) = (&buy_sell.sell_state_changes, &sim_result.pool_address) {
                sell_tax = calculate_sell_tax(sell_changes, pool_addr, &buyer_address);
                if sell_tax.is_none() {
                    debug!("Sell tax calculation returned None for pool {:?}", pool_addr);
                }
            } else {
                info!("Missing data for sell tax calculation - sell_changes: {}, pool_addr: {}", 
                    buy_sell.sell_state_changes.is_some(), 
                    sim_result.pool_address.is_some());
            }
            
            (buy_tax, sell_tax)
        } else {
            (None, None)
        };

        // Always log the tax calculation results
        self.log_tax_detection(&token_address, sim_result, calculated_buy_tax, calculated_sell_tax);

        // Check for high taxes or honeypot patterns - consolidated detection
        let cant_sell = sim_result.buy_sell_result.as_ref().map(|bs| !bs.can_sell).unwrap_or(false);
        let buy_tax_exceeds_threshold = calculated_buy_tax.unwrap_or(0.0) >= self.config.max_acceptable_buy_tax as f64;
        let sell_tax_exceeds_threshold = calculated_sell_tax.unwrap_or(0.0) >= self.config.max_acceptable_sell_tax as f64;
        
        // Generate consolidated high tax/honeypot signal if any condition is met
        if buy_tax_exceeds_threshold || sell_tax_exceeds_threshold || cant_sell {
            let details = match (buy_tax_exceeds_threshold, sell_tax_exceeds_threshold, cant_sell) {
                (_, _, true) => "Honeypot detected! Cannot sell tokens".to_string(),
                (true, true, _) => format!("High taxes - Buy: {:.1}%, Sell: {:.1}%", 
                    calculated_buy_tax.unwrap_or(0.0), calculated_sell_tax.unwrap_or(0.0)),
                (true, false, _) => format!("High buy tax: {:.1}%", calculated_buy_tax.unwrap_or(0.0)),
                (false, true, _) => format!("High sell tax: {:.1}%", calculated_sell_tax.unwrap_or(0.0)),
                _ => "Tax issue detected".to_string(),
            };

            signals.push(TaxSignal {
                token_address: token_address.clone(),
                signal_type: TaxSignalType::HighTaxOrHoneypot { 
                    buy_tax_exceeds_threshold, 
                    sell_tax_exceeds_threshold, 
                    cant_sell 
                },
                buy_tax: calculated_buy_tax,
                sell_tax: calculated_sell_tax,
                details,
                confidence: if cant_sell { 0.95 } else { 0.9 },
            });
            
            // If can't sell (honeypot), return early (don't check other patterns)
            if cant_sell {
                return signals;
            }
        }

        // Check for suspicious patterns (only if no high tax/honeypot detected)
        if let (Some(buy_tax), Some(sell_tax)) = (calculated_buy_tax, calculated_sell_tax) {
            // Low buy tax with high sell tax difference is suspicious
            let suspicious_difference_threshold = 20.0; // Configurable if needed later
            if buy_tax <= 5.0 && sell_tax >= buy_tax + suspicious_difference_threshold {
                signals.push(TaxSignal {
                    token_address: token_address.clone(),
                    signal_type: TaxSignalType::SuspiciousPattern,
                    buy_tax: Some(buy_tax),
                    sell_tax: Some(sell_tax),
                    details: format!("Suspicious tax pattern - Buy: {:.1}% (low), Sell: {:.1}% (high)", buy_tax, sell_tax),
                    confidence: 0.8,
                });
            }
        }

        // Log summary if any signals detected
        if !signals.is_empty() {
            // Log to file if path is configured
            if let Some(ref log_path) = self.log_file_path {
                for signal in &signals {
                    if let Ok(mut file) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(log_path)
                    {
                        let timestamp = chrono::Local::now();
                        let buy_tax_str = signal.buy_tax
                            .map(|t| format!("{:.1}%", t))
                            .unwrap_or_else(|| "None".to_string());
                        let sell_tax_str = signal.sell_tax
                            .map(|t| format!("{:.1}%", t))
                            .unwrap_or_else(|| "None".to_string());
                            
                        writeln!(file, "[{}] TX: {} | Token: {} | Type: {:?} | Buy Tax: {} | Sell Tax: {} | Confidence: {:.2} | Details: {}",
                            timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                            sim_result.request.tx.hash,
                            signal.token_address,
                            signal.signal_type,
                            buy_tax_str,
                            sell_tax_str,
                            signal.confidence,
                            signal.details
                        ).ok();
                    }
                }
            }
        }

        signals
    }

    /// Log tax detection results for debugging and monitoring
    fn log_tax_detection(&self, token_address: &str, sim_result: &SimulationResult, buy_tax: Option<f64>, sell_tax: Option<f64>) {
        if let Some(ref log_path) = self.log_file_path {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
            {
                let timestamp = chrono::Local::now();
                
                // Extract pool address if available
                let pool_address = sim_result.pool_address
                    .map(|addr| format!("0x{}", hex::encode(addr)))
                    .unwrap_or_else(|| "unknown".to_string());
                
                // Extract buy/sell capabilities
                let (can_buy, can_sell) = if let Some(buy_sell) = &sim_result.buy_sell_result {
                    (buy_sell.can_buy, buy_sell.can_sell)
                } else {
                    (false, false)
                };
                
                writeln!(file, 
                    "[{}] TAX_DETECTION | TX: {} | Token: {} | Pool: {} | can_buy: {} | can_sell: {} | buy_tax: {:.1}% | sell_tax: {:.1}%",
                    timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                    sim_result.request.tx.hash,
                    token_address,
                    pool_address,
                    can_buy,
                    can_sell,
                    buy_tax.unwrap_or(-1.0),  // -1 indicates not calculated
                    sell_tax.unwrap_or(-1.0)   // -1 indicates not calculated
                ).ok();
            }
        }
    }


    /// Check if taxes indicate a honeypot
    pub fn is_honeypot(&self, _buy_tax: Option<f64>, _sell_tax: Option<f64>, can_sell: bool) -> bool {
        // Primary honeypot indicator: can't sell at all
        !can_sell
    }

    /// Determine if trading should be enabled based on taxes
    pub fn should_enable_trading(&self, buy_tax: Option<f64>, sell_tax: Option<f64>) -> bool {
        // Don't enable if taxes are too high
        if let Some(sell) = sell_tax {
            if sell >= self.config.max_acceptable_sell_tax as f64 {
                return false;
            }
        }
        
        if let Some(buy) = buy_tax {
            if buy >= self.config.max_acceptable_buy_tax as f64 {
                return false;
            }
        }
        
        true
    }
}

impl Default for TaxDetector {
    fn default() -> Self {
        use crate::config::TaxDetectionConfig;
        Self::new(TaxDetectionConfig::default())
    }
}