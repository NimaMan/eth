use crate::config::TaxDetectionConfig;
/// Tax Detector
///
/// Analyzes state changes to calculate taxes and detect tax-related signals.
/// This detector centralizes all tax calculation and detection logic.
///
/// Detects:
/// - High buy/sell taxes
/// - Tax changes (when compared to previous state)
/// - Suspicious tax patterns
use crate::simulator::SimulationResult;
use chrono;
use eth_token::pools::TaxBucket;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TaxSignal {
    pub token_address: String,
    pub signal_type: TaxSignalType,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub buy_tax_bucket_from: Option<String>,
    pub buy_tax_bucket_to: Option<String>,
    pub sell_tax_bucket_from: Option<String>,
    pub sell_tax_bucket_to: Option<String>,
    pub combined_tax_bucket_from: Option<String>,
    pub combined_tax_bucket_to: Option<String>,
    pub details: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaxSignalType {
    /// Tax moved into a risky bucket or exceeded configured thresholds.
    TaxBucketRisk {
        buy_tax_exceeds_threshold: bool,
        sell_tax_exceeds_threshold: bool,
    },
    /// Tax change detected (requires historical data)
    TaxChange {
        before_buy: f64,
        before_sell: f64,
        after_buy: f64,
        after_sell: f64,
    },
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
        let buy_sell = match sim_result.buy_sell_result() {
            Some(bs) => bs,
            None => return signals,
        };

        // Extract token and pool addresses
        let (token_address, _pool_address) = match &sim_result.request.category {
            crate::tx_router::TransactionCategory::CreatorTransaction { target_token, .. } => {
                if let Some(token) = target_token {
                    // Need to find pool address - for now we'll use the one from simulation
                    (token.clone(), None::<alloy_primitives::Address>)
                } else {
                    return signals;
                }
            }
            crate::tx_router::TransactionCategory::ContractCreation {
                contract_address, ..
            } => (contract_address.clone(), None),
            _ => return signals,
        };

        // Get tax values directly from simulation result (calculated in simulation_manager)
        let calculated_buy_tax = buy_sell.buy_tax;
        let calculated_sell_tax = buy_sell.sell_tax;
        let buy_tax_bucket = TaxBucket::from_percent(calculated_buy_tax);
        let sell_tax_bucket = TaxBucket::from_percent(calculated_sell_tax);
        let combined_tax_bucket = TaxBucket::combined(calculated_buy_tax, calculated_sell_tax);
        let buy_tax_bucket_key = tax_bucket_key(buy_tax_bucket);
        let sell_tax_bucket_key = tax_bucket_key(sell_tax_bucket);
        let combined_tax_bucket_key = tax_bucket_key(combined_tax_bucket);

        // Always log the tax calculation results
        self.log_tax_detection(
            &token_address,
            sim_result,
            calculated_buy_tax,
            calculated_sell_tax,
        );

        // Check for high taxes. Sell-blocked/honeypot is emitted as a separate
        // semantic signal by SignalManager, not as a tax signal.
        let buy_tax_exceeds_threshold =
            calculated_buy_tax.unwrap_or(0.0) >= self.config.max_acceptable_buy_tax as f64;
        let sell_tax_exceeds_threshold =
            calculated_sell_tax.unwrap_or(0.0) >= self.config.max_acceptable_sell_tax as f64;
        let bucket_is_risky = matches!(
            combined_tax_bucket,
            TaxBucket::HighTax | TaxBucket::ExtremeTax
        );

        if buy_tax_exceeds_threshold || sell_tax_exceeds_threshold || bucket_is_risky {
            let details = format!(
                "Tax risk - Buy: {:.1}% ({buy_tax_bucket_key}), Sell: {:.1}% ({sell_tax_bucket_key}), Combined: {combined_tax_bucket_key}",
                calculated_buy_tax.unwrap_or(0.0),
                calculated_sell_tax.unwrap_or(0.0)
            );

            signals.push(TaxSignal {
                token_address: token_address.clone(),
                signal_type: TaxSignalType::TaxBucketRisk {
                    buy_tax_exceeds_threshold,
                    sell_tax_exceeds_threshold,
                },
                buy_tax: calculated_buy_tax,
                sell_tax: calculated_sell_tax,
                buy_tax_bucket_from: None,
                buy_tax_bucket_to: Some(buy_tax_bucket_key.to_string()),
                sell_tax_bucket_from: None,
                sell_tax_bucket_to: Some(sell_tax_bucket_key.to_string()),
                combined_tax_bucket_from: None,
                combined_tax_bucket_to: Some(combined_tax_bucket_key.to_string()),
                details,
                confidence: 0.9,
            });
        }

        // Check for suspicious patterns.
        if let (Some(buy_tax), Some(sell_tax)) = (calculated_buy_tax, calculated_sell_tax) {
            // Low buy tax with high sell tax difference is suspicious
            let suspicious_difference_threshold = 20.0; // Configurable if needed later
            if buy_tax <= 5.0 && sell_tax >= buy_tax + suspicious_difference_threshold {
                signals.push(TaxSignal {
                    token_address: token_address.clone(),
                    signal_type: TaxSignalType::SuspiciousPattern,
                    buy_tax: Some(buy_tax),
                    sell_tax: Some(sell_tax),
                    buy_tax_bucket_from: None,
                    buy_tax_bucket_to: Some(
                        tax_bucket_key(TaxBucket::from_percent(Some(buy_tax))).to_string(),
                    ),
                    sell_tax_bucket_from: None,
                    sell_tax_bucket_to: Some(
                        tax_bucket_key(TaxBucket::from_percent(Some(sell_tax))).to_string(),
                    ),
                    combined_tax_bucket_from: None,
                    combined_tax_bucket_to: Some(
                        tax_bucket_key(TaxBucket::combined(Some(buy_tax), Some(sell_tax)))
                            .to_string(),
                    ),
                    details: format!(
                        "Suspicious tax pattern - Buy: {:.1}% (low), Sell: {:.1}% (high)",
                        buy_tax, sell_tax
                    ),
                    confidence: 0.8,
                });
            }
        }

        // Log summary if any signals detected
        if !signals.is_empty() {
            // Log to file if path is configured
            if let Some(ref log_path) = self.log_file_path {
                for signal in &signals {
                    if let Ok(mut file) =
                        OpenOptions::new().create(true).append(true).open(log_path)
                    {
                        let timestamp = chrono::Local::now();
                        let buy_tax_str = signal
                            .buy_tax
                            .map(|t| format!("{:.1}%", t))
                            .unwrap_or_else(|| "None".to_string());
                        let sell_tax_str = signal
                            .sell_tax
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
    fn log_tax_detection(
        &self,
        token_address: &str,
        sim_result: &SimulationResult,
        buy_tax: Option<f64>,
        sell_tax: Option<f64>,
    ) {
        if let Some(ref log_path) = self.log_file_path {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
                let timestamp = chrono::Local::now();

                // Extract pool address if available
                let pool_address = sim_result
                    .pool_address
                    .map(|addr| format!("0x{}", hex::encode(addr)))
                    .unwrap_or_else(|| "unknown".to_string());

                // Extract buy/sell capabilities and error messages
                let (can_buy, can_approve, can_sell, buy_tax_error, sell_tax_error) =
                    if let Some(buy_sell) = sim_result.buy_sell_result() {
                        (
                            buy_sell.can_buy,
                            buy_sell.can_approve,
                            buy_sell.can_sell,
                            buy_sell.buy_tax_error.clone(),
                            buy_sell.sell_tax_error.clone(),
                        )
                    } else {
                        (false, false, false, None, None)
                    };

                // Format buy tax: show percentage if calculated, or error if failed
                let buy_tax_str = match buy_tax {
                    Some(tax) => format!("{:.1}%", tax),
                    None => match buy_tax_error {
                        Some(error) => format!("ERROR: {}", error),
                        None => "ERROR: Unknown".to_string(),
                    },
                };

                // Format sell tax: show percentage if calculated, or error if failed
                let sell_tax_str = match sell_tax {
                    Some(tax) => format!("{:.1}%", tax),
                    None => match sell_tax_error {
                        Some(error) => format!("ERROR: {}", error),
                        None => "ERROR: Unknown".to_string(),
                    },
                };

                writeln!(
                    file,
                    "[{}] TAX_DETECTION | TX: {} | Token: {} | Pool: {} | can_buy: {} | can_approve: {} | can_sell: {} | buy_tax: {} | sell_tax: {}",
                    timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
                    sim_result.request.tx.hash,
                    token_address,
                    pool_address,
                    can_buy,
                    can_approve,
                    can_sell,
                    buy_tax_str,
                    sell_tax_str
                )
                .ok();
            }
        }
    }

    /// Check if simulation indicates a buy-then-stuck honeypot.
    pub fn is_honeypot(
        &self,
        _buy_tax: Option<f64>,
        _sell_tax: Option<f64>,
        can_buy: bool,
        can_approve: bool,
        can_sell: bool,
    ) -> bool {
        can_buy && can_approve && !can_sell
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

fn tax_bucket_key(bucket: TaxBucket) -> &'static str {
    match bucket {
        TaxBucket::Unknown => "unknown",
        TaxBucket::NoTax => "no_tax",
        TaxBucket::LowTax => "low_tax",
        TaxBucket::ModerateTax => "moderate_tax",
        TaxBucket::HighTax => "high_tax",
        TaxBucket::ExtremeTax => "extreme_tax",
    }
}

impl Default for TaxDetector {
    fn default() -> Self {
        use crate::config::TaxDetectionConfig;
        Self::new(TaxDetectionConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::TaxDetector;

    #[test]
    fn honeypot_requires_buy_and_approve_before_failed_sell() {
        let detector = TaxDetector::default();

        assert!(detector.is_honeypot(None, None, true, true, false));
        assert!(!detector.is_honeypot(None, None, false, true, false));
        assert!(!detector.is_honeypot(None, None, true, false, false));
        assert!(!detector.is_honeypot(None, None, true, true, true));
    }
}
