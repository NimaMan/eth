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
use crate::token_parameter_extraction::{calculate_buy_tax, calculate_sell_tax};
use tracing::{info, debug};
use std::collections::HashMap;
use alloy_primitives::Address;
use reth_tx_simulator::AddressStateChange;
use hex;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

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
    /// High tax detected (but not honeypot level)
    HighTax { buy: bool, sell: bool },
    /// Honeypot pattern detected (sell tax > 50%)
    Honeypot,
    /// Tax change detected (requires historical data)
    TaxChange { before_buy: f64, before_sell: f64, after_buy: f64, after_sell: f64 },
    /// Suspicious tax pattern (e.g., low buy, high sell)
    SuspiciousPattern,
}

pub struct TaxDetector {
    /// Threshold for high tax warning (default: 25%)
    high_tax_threshold: f64,
    /// Threshold for honeypot detection (default: 50%)
    honeypot_threshold: f64,
    /// Threshold for suspicious tax difference (default: 20%)
    suspicious_difference_threshold: f64,
    /// Path to the log file
    log_file_path: Option<PathBuf>,
}

impl TaxDetector {
    pub fn new() -> Self {
        Self {
            high_tax_threshold: 25.0,
            honeypot_threshold: 50.0,
            suspicious_difference_threshold: 20.0,
            log_file_path: None,
        }
    }
    
    pub fn with_log_path(log_path: PathBuf) -> Self {
        Self {
            high_tax_threshold: 25.0,
            honeypot_threshold: 50.0,
            suspicious_difference_threshold: 20.0,
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
            }
            
            // Calculate sell tax if we have the state changes and addresses
            if let (Some(sell_changes), Some(pool_addr)) = (&buy_sell.sell_state_changes, &sim_result.pool_address) {
                sell_tax = calculate_sell_tax(sell_changes, pool_addr, &buyer_address);
            }
            
            (buy_tax, sell_tax)
        } else {
            (None, None)
        };

        // Check for honeypot - either can't sell or sell tax is too high
        let is_honeypot = if let Some(buy_sell) = &sim_result.buy_sell_result {
            !buy_sell.can_sell || calculated_sell_tax.unwrap_or(0.0) >= self.honeypot_threshold
        } else {
            false
        };
        
        if is_honeypot {
            let (details, buy_tax_for_signal, sell_tax_for_signal) = if let Some(buy_sell) = &sim_result.buy_sell_result {
                if !buy_sell.can_sell {
                    // Can't sell, so sell tax is None (not 0)
                    let buy_tax = if buy_sell.can_buy { calculated_buy_tax } else { None };
                    ("Honeypot detected! Cannot sell tokens".to_string(), buy_tax, None)
                } else {
                    // High sell tax case
                    (format!("Honeypot detected! Sell tax: {:.1}%", calculated_sell_tax.unwrap_or(100.0)), 
                     calculated_buy_tax, 
                     calculated_sell_tax)
                }
            } else {
                ("Honeypot detected!".to_string(), None, None)
            };
            
            signals.push(TaxSignal {
                token_address: token_address.clone(),
                signal_type: TaxSignalType::Honeypot,
                buy_tax: buy_tax_for_signal,
                sell_tax: sell_tax_for_signal,
                details,
                confidence: 0.95,
            });
            return signals; // Honeypot overrides other signals
        }

        // Check for high taxes
        let high_buy = calculated_buy_tax.unwrap_or(0.0) >= self.high_tax_threshold;
        let high_sell = calculated_sell_tax.unwrap_or(0.0) >= self.high_tax_threshold;
        
        if high_buy || high_sell {
            let details = match (high_buy, high_sell) {
                (true, true) => format!("High taxes detected - Buy: {:.1}%, Sell: {:.1}%", 
                    calculated_buy_tax.unwrap_or(0.0), calculated_sell_tax.unwrap_or(0.0)),
                (true, false) => format!("High buy tax: {:.1}%", calculated_buy_tax.unwrap_or(0.0)),
                (false, true) => format!("High sell tax: {:.1}%", calculated_sell_tax.unwrap_or(0.0)),
                _ => unreachable!(),
            };

            signals.push(TaxSignal {
                token_address: token_address.clone(),
                signal_type: TaxSignalType::HighTax { buy: high_buy, sell: high_sell },
                buy_tax: calculated_buy_tax,
                sell_tax: calculated_sell_tax,
                details,
                confidence: 0.9,
            });
        }

        // Check for suspicious patterns
        if let (Some(buy_tax), Some(sell_tax)) = (calculated_buy_tax, calculated_sell_tax) {
            // Low buy tax with high sell tax is suspicious
            if buy_tax <= 5.0 && sell_tax >= buy_tax + self.suspicious_difference_threshold {
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
            info!("💸 Tax detector found {} signals for token {}", signals.len(), token_address);
            
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

    /// Calculate taxes directly from state changes
    /// This can be used when we want to bypass pre-calculated values
    pub fn calculate_taxes_from_state_changes(
        &self,
        state_changes: &HashMap<Address, AddressStateChange>,
        pool_address: &Address,
        buyer_address: &Address,
        token_address: &Address,
    ) -> (Option<f64>, Option<f64>) {
        // Calculate buy tax
        let buy_tax = calculate_buy_tax(state_changes, pool_address, buyer_address, token_address);
        
        // For sell tax, we'd need the sell transaction state changes
        // This is just for buy transaction
        let sell_tax = None;
        
        debug!("Calculated taxes - Buy: {:?}%, Sell: {:?}%", buy_tax, sell_tax);
        
        (buy_tax, sell_tax)
    }

    /// Check if taxes indicate a honeypot
    pub fn is_honeypot(&self, buy_tax: Option<f64>, sell_tax: Option<f64>, can_sell: bool) -> bool {
        // If sell fails, it's a honeypot
        if !can_sell {
            return true;
        }
        
        // If sell tax is above honeypot threshold
        if let Some(sell) = sell_tax {
            if sell >= self.honeypot_threshold {
                return true;
            }
        }
        
        false
    }

    /// Determine if trading should be enabled based on taxes
    pub fn should_enable_trading(&self, buy_tax: Option<f64>, sell_tax: Option<f64>) -> bool {
        // Don't enable if it's a honeypot
        if let Some(sell) = sell_tax {
            if sell >= self.honeypot_threshold {
                return false;
            }
        }
        
        // Don't enable if taxes are extremely high
        if let Some(buy) = buy_tax {
            if buy >= self.honeypot_threshold {
                return false;
            }
        }
        
        true
    }
}

impl Default for TaxDetector {
    fn default() -> Self {
        Self::new()
    }
}