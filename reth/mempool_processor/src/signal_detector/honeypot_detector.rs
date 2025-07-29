/// Honeypot Detector
/// 
/// Detects honeypot tokens using buy/sell simulation results

use crate::simulator::{BuySellResult, SimulationResult};
use tracing::{info, debug};

#[derive(Debug, Clone)]
pub struct HoneypotSignal {
    pub token_address: String,
    pub detection_method: HoneypotDetectionMethod,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub confidence: f64,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HoneypotDetectionMethod {
    /// Sell transaction fails
    SellFails,
    /// Extremely high sell tax (>50%)
    HighSellTax,
    /// Buy works but sell returns 0
    ZeroSellReturn,
    /// Other suspicious pattern
    SuspiciousPattern(String),
}

pub struct HoneypotDetector {
    /// Threshold for high tax detection
    high_tax_threshold: f64,
    /// Threshold for suspicious tax difference
    tax_difference_threshold: f64,
}

impl HoneypotDetector {
    pub fn new() -> Self {
        Self {
            high_tax_threshold: 50.0,
            tax_difference_threshold: 30.0,
        }
    }

    /// Detect honeypot from simulation results
    pub fn detect(&self, sim_result: &SimulationResult) -> Option<HoneypotSignal> {
        let buy_sell = sim_result.buy_sell_result.as_ref()?;
        
        // Extract token address from request
        let token_address = match &sim_result.request.category {
            crate::tx_router::TransactionCategory::CreatorTransaction { target_token, .. } => {
                target_token.as_ref()?.clone()
            }
            _ => return None,
        };

        // Check for honeypot patterns
        if buy_sell.is_honeypot {
            return Some(self.create_honeypot_signal(
                token_address,
                buy_sell,
                HoneypotDetectionMethod::SellFails,
                1.0,
                "Token is confirmed honeypot - sells fail".to_string(),
            ));
        }

        // Check for high sell tax
        if let Some(sell_tax) = buy_sell.sell_tax {
            if sell_tax > self.high_tax_threshold {
                return Some(self.create_honeypot_signal(
                    token_address,
                    buy_sell,
                    HoneypotDetectionMethod::HighSellTax,
                    0.9,
                    format!("Extremely high sell tax: {:.1}%", sell_tax),
                ));
            }
        }

        // Check for zero sell return
        if buy_sell.can_buy && buy_sell.can_sell {
            if let Some(eth_received) = buy_sell.eth_received_on_sell {
                if eth_received < 0.001 {
                    return Some(self.create_honeypot_signal(
                        token_address,
                        buy_sell,
                        HoneypotDetectionMethod::ZeroSellReturn,
                        0.95,
                        "Sell returns almost nothing".to_string(),
                    ));
                }
            }
        }

        // Check for suspicious tax patterns
        if let (Some(buy_tax), Some(sell_tax)) = (buy_sell.buy_tax, buy_sell.sell_tax) {
            // Low/zero buy tax with high sell tax is suspicious
            if buy_tax <= 5.0 && sell_tax > 30.0 {
                return Some(self.create_honeypot_signal(
                    token_address,
                    buy_sell,
                    HoneypotDetectionMethod::SuspiciousPattern("Low buy, high sell".to_string()),
                    0.7,
                    format!("Suspicious tax pattern: buy {:.1}%, sell {:.1}%", buy_tax, sell_tax),
                ));
            }
        }

        None
    }

    fn create_honeypot_signal(
        &self,
        token_address: String,
        buy_sell: &BuySellResult,
        method: HoneypotDetectionMethod,
        confidence: f64,
        details: String,
    ) -> HoneypotSignal {
        info!("🍯 Honeypot detected: {} - {}", token_address, details);
        
        HoneypotSignal {
            token_address,
            detection_method: method,
            buy_tax: buy_sell.buy_tax,
            sell_tax: buy_sell.sell_tax,
            can_buy: buy_sell.can_buy,
            can_sell: buy_sell.can_sell,
            confidence,
            details,
        }
    }
}