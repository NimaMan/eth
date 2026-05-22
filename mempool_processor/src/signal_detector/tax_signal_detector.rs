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
use eth_token::pools::TaxBucket;

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
}

impl TaxDetector {
    pub fn new(config: TaxDetectionConfig) -> Self {
        Self { config }
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
        let buy_tax_bucket_key = buy_tax_bucket.key();
        let sell_tax_bucket_key = sell_tax_bucket.key();
        let combined_tax_bucket_key = combined_tax_bucket.key();

        // Check for high taxes. Sell-blocked/honeypot is emitted as a separate
        // semantic signal by SignalManager, not as a tax signal.
        let buy_tax_exceeds_threshold =
            calculated_buy_tax.unwrap_or(0.0) >= self.config.max_acceptable_buy_tax as f64;
        let sell_tax_exceeds_threshold =
            calculated_sell_tax.unwrap_or(0.0) >= self.config.max_acceptable_sell_tax as f64;
        let bucket_is_risky = combined_tax_bucket.is_risky();

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
                        TaxBucket::from_percent(Some(buy_tax)).key().to_string(),
                    ),
                    sell_tax_bucket_from: None,
                    sell_tax_bucket_to: Some(
                        TaxBucket::from_percent(Some(sell_tax)).key().to_string(),
                    ),
                    combined_tax_bucket_from: None,
                    combined_tax_bucket_to: Some(
                        TaxBucket::combined(Some(buy_tax), Some(sell_tax))
                            .key()
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

        signals
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
