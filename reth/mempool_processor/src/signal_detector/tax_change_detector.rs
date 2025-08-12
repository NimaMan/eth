/// Tax Change Detector
/// 
/// Detects changes in token buy/sell taxes

use crate::simulator::{BuySellResult, SimulationResult};
use crate::function_detector::CreatorFunctionType;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct TaxChangeSignal {
    pub token_address: String,
    pub before: TaxInfo,
    pub after: TaxInfo,
    pub changer_address: String,
    pub function_called: String,
    pub is_honeypot_after: bool,
    pub risk_level: TaxRiskLevel,
    pub confidence: f64,
    pub details: String,
}

#[derive(Debug, Clone)]
pub struct TaxInfo {
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaxRiskLevel {
    /// Tax increased significantly
    Critical,
    /// Tax increased moderately
    High,
    /// Tax changed but not critically
    Medium,
    /// Tax decreased or minimal change
    Low,
}

pub struct TaxChangeDetector {
    /// Threshold for critical tax increase
    critical_increase_threshold: f64,
    /// Threshold for high tax level
    high_tax_threshold: f64,
}

impl TaxChangeDetector {
    pub fn new() -> Self {
        Self {
            critical_increase_threshold: 20.0, // 20% increase
            high_tax_threshold: 30.0,         // 30% tax
        }
    }

    /// Detect tax changes from simulation
    pub fn detect(&self, sim_result: &SimulationResult) -> Option<TaxChangeSignal> {
        // Only detect for creator transactions that modify taxes
        let (creator, function_type, target_token) = match &sim_result.request.category {
            crate::tx_router::TransactionCategory::CreatorTransaction { 
                creator, 
                function_type, 
                target_token,
                .. 
            } => {
                if !matches!(function_type, CreatorFunctionType::TaxModification) {
                    return None;
                }
                (creator, function_type, target_token.as_ref()?)
            }
            _ => return None,
        };

        // Get buy/sell result
        let buy_sell = sim_result.buy_sell_result.as_ref()?;

        // In real implementation, we'd get "before" state from cache/database
        // For now, simulate with placeholder
        let before = TaxInfo {
            buy_tax: Some(5.0),
            sell_tax: Some(5.0),
        };

        let after = TaxInfo {
            buy_tax: None, // TODO: Calculate from state changes
            sell_tax: None, // TODO: Calculate from state changes
        };

        // Calculate risk level
        let risk_level = self.calculate_risk_level(&before, &after);

        // Generate details
        let details = self.generate_details(&before, &after, false); // TODO: Calculate honeypot from state changes

        info!("📊 Tax change detected for {}: {}", target_token, details);

        Some(TaxChangeSignal {
            token_address: target_token.clone(),
            before,
            after,
            changer_address: creator.clone(),
            function_called: format!("{:?}", function_type),
            is_honeypot_after: false, // TODO: Calculate from state changes
            risk_level,
            confidence: 0.9, // High confidence since we simulated it
            details,
        })
    }

    fn calculate_risk_level(&self, before: &TaxInfo, after: &TaxInfo) -> TaxRiskLevel {
        // Check if it became a honeypot
        if let Some(after_sell) = after.sell_tax {
            if after_sell > 90.0 {
                return TaxRiskLevel::Critical;
            }
        }

        // Calculate tax increases
        let buy_increase = match (before.buy_tax, after.buy_tax) {
            (Some(b), Some(a)) => a - b,
            _ => 0.0,
        };

        let sell_increase = match (before.sell_tax, after.sell_tax) {
            (Some(b), Some(a)) => a - b,
            _ => 0.0,
        };

        // Determine risk level
        if sell_increase > self.critical_increase_threshold || 
           buy_increase > self.critical_increase_threshold {
            TaxRiskLevel::Critical
        } else if after.sell_tax.unwrap_or(0.0) > self.high_tax_threshold ||
                  after.buy_tax.unwrap_or(0.0) > self.high_tax_threshold {
            TaxRiskLevel::High
        } else if sell_increase > 5.0 || buy_increase > 5.0 {
            TaxRiskLevel::Medium
        } else {
            TaxRiskLevel::Low
        }
    }

    fn generate_details(&self, before: &TaxInfo, after: &TaxInfo, _is_honeypot: bool) -> String {
        let mut details = Vec::new();

        if let (Some(b_buy), Some(a_buy)) = (before.buy_tax, after.buy_tax) {
            if (a_buy - b_buy).abs() > 0.1 {
                details.push(format!("Buy tax: {:.1}% → {:.1}%", b_buy, a_buy));
            }
        }

        if let (Some(b_sell), Some(a_sell)) = (before.sell_tax, after.sell_tax) {
            if (a_sell - b_sell).abs() > 0.1 {
                details.push(format!("Sell tax: {:.1}% → {:.1}%", b_sell, a_sell));
            }
        }

        if _is_honeypot {
            details.push("TOKEN IS NOW A HONEYPOT!".to_string());
        }

        if details.is_empty() {
            "Tax values updated".to_string()
        } else {
            details.join(", ")
        }
    }
}