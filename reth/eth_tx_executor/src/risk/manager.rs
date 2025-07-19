//! Risk Manager Module
//!
//! Provides essential risk checks for trade execution

use ethers::types::{Address, U256};
use tracing::info;
use crate::alert_processor::Alert;

/// Risk decision based on essential checks
#[derive(Debug, Clone)]
pub enum RiskDecision {
    /// Allow the trade to proceed
    Allow,
    /// Block the trade entirely
    Block { reason: String },
}

/// Risk configuration
#[derive(Debug, Clone)]
pub struct RiskConfig {
    /// Maximum gas cost as percentage of trade value
    pub max_gas_cost_percent: f64,
    /// Maximum acceptable slippage percentage
    pub max_slippage_percent: f64,
    /// Minimum ETH balance to maintain (for gas buffer)
    pub min_eth_balance: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_gas_cost_percent: 5.0,    // 5% max gas cost
            max_slippage_percent: 3.0,    // 3% max slippage
            min_eth_balance: 0.01,        // Keep 0.01 ETH for gas
        }
    }
}

/// Risk manager handles essential trade safety checks
pub struct RiskManager {
    config: RiskConfig,
}

impl RiskManager {
    pub fn new(config: RiskConfig) -> Self {
        Self { config }
    }
    
    /// Evaluate if a trade should be allowed based on essential checks
    pub fn evaluate_trade_risk(
        &self,
        _alert: &Alert,
        trade_amount_eth: f64,
        _token_address: Address,
    ) -> RiskDecision {
        // For now, just check that trade amount is reasonable
        if trade_amount_eth <= 0.0 {
            return RiskDecision::Block {
                reason: "Invalid trade amount".to_string(),
            };
        }
        
        info!("Trade approved: {} ETH", trade_amount_eth);
        RiskDecision::Allow
    }
    
    /// Check if we have sufficient funds for the trade
    pub fn check_sufficient_funds(
        &self,
        available_balance: U256,
        required_amount: U256,
        gas_cost_eth: f64,
    ) -> RiskDecision {
        // Calculate total needed including gas
        let gas_cost_wei = U256::from((gas_cost_eth * 1e18) as u128);
        let min_balance_wei = U256::from((self.config.min_eth_balance * 1e18) as u128);
        let total_needed = required_amount + gas_cost_wei + min_balance_wei;
        
        if available_balance < total_needed {
            let available_eth = available_balance.as_u128() as f64 / 1e18;
            let needed_eth = total_needed.as_u128() as f64 / 1e18;
            
            return RiskDecision::Block {
                reason: format!(
                    "Insufficient funds: have {:.4} ETH, need {:.4} ETH",
                    available_eth, needed_eth
                ),
            };
        }
        
        RiskDecision::Allow
    }
    
    /// Check if gas cost is reasonable relative to trade size
    pub fn check_gas_cost(
        &self,
        trade_amount_eth: f64,
        gas_cost_eth: f64,
    ) -> RiskDecision {
        if trade_amount_eth <= 0.0 {
            return RiskDecision::Block {
                reason: "Invalid trade amount".to_string(),
            };
        }
        
        let gas_percentage = (gas_cost_eth / trade_amount_eth) * 100.0;
        
        if gas_percentage > self.config.max_gas_cost_percent {
            return RiskDecision::Block {
                reason: format!(
                    "Gas cost too high: {:.1}% of trade (max: {:.1}%)",
                    gas_percentage, self.config.max_gas_cost_percent
                ),
            };
        }
        
        info!("Gas cost acceptable: {:.1}% of trade value", gas_percentage);
        RiskDecision::Allow
    }
    
    /// Check if slippage is within acceptable bounds
    pub fn check_slippage(
        &self,
        expected_output: U256,
        actual_output: U256,
    ) -> RiskDecision {
        if expected_output == U256::zero() {
            return RiskDecision::Block {
                reason: "Invalid expected output".to_string(),
            };
        }
        
        // Calculate slippage percentage
        let expected_f64 = expected_output.as_u128() as f64;
        let actual_f64 = actual_output.as_u128() as f64;
        let slippage_percent = ((expected_f64 - actual_f64) / expected_f64) * 100.0;
        
        if slippage_percent > self.config.max_slippage_percent {
            return RiskDecision::Block {
                reason: format!(
                    "Slippage too high: {:.1}% (max: {:.1}%)",
                    slippage_percent, self.config.max_slippage_percent
                ),
            };
        }
        
        if slippage_percent > 0.0 {
            info!("Slippage acceptable: {:.1}%", slippage_percent);
        }
        
        RiskDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sufficient_funds_check() {
        let config = RiskConfig::default();
        let manager = RiskManager::new(config);
        
        // Test: Sufficient funds
        let available = U256::from((0.05 * 1e18) as u128); // 0.05 ETH
        let required = U256::from((0.01 * 1e18) as u128);  // 0.01 ETH
        let gas_cost = 0.0002; // 0.0002 ETH
        
        match manager.check_sufficient_funds(available, required, gas_cost) {
            RiskDecision::Allow => {}, // Expected
            RiskDecision::Block { reason } => panic!("Should allow: {}", reason),
        }
        
        // Test: Insufficient funds
        let available = U256::from((0.015 * 1e18) as u128); // 0.015 ETH
        
        match manager.check_sufficient_funds(available, required, gas_cost) {
            RiskDecision::Block { reason } => {
                assert!(reason.contains("Insufficient funds"));
            }
            RiskDecision::Allow => panic!("Should block insufficient funds"),
        }
    }
    
    #[test]
    fn test_gas_cost_check() {
        let config = RiskConfig {
            max_gas_cost_percent: 5.0,
            ..Default::default()
        };
        let manager = RiskManager::new(config);
        
        // Test: Acceptable gas cost (2%)
        let trade_amount = 0.01;
        let gas_cost = 0.0002;
        
        match manager.check_gas_cost(trade_amount, gas_cost) {
            RiskDecision::Allow => {}, // Expected
            RiskDecision::Block { reason } => panic!("Should allow: {}", reason),
        }
        
        // Test: High gas cost (30%)
        let high_gas_cost = 0.003;
        
        match manager.check_gas_cost(trade_amount, high_gas_cost) {
            RiskDecision::Block { reason } => {
                assert!(reason.contains("Gas cost too high"));
                assert!(reason.contains("30.0%"));
            }
            RiskDecision::Allow => panic!("Should block high gas cost"),
        }
    }
    
    #[test]
    fn test_slippage_check() {
        let config = RiskConfig {
            max_slippage_percent: 3.0,
            ..Default::default()
        };
        let manager = RiskManager::new(config);
        
        // Test: Acceptable slippage (1%)
        let expected = U256::from(1000);
        let actual = U256::from(990);
        
        match manager.check_slippage(expected, actual) {
            RiskDecision::Allow => {}, // Expected
            RiskDecision::Block { reason } => panic!("Should allow: {}", reason),
        }
        
        // Test: High slippage (15%)
        let actual_low = U256::from(850);
        
        match manager.check_slippage(expected, actual_low) {
            RiskDecision::Block { reason } => {
                assert!(reason.contains("Slippage too high"));
                assert!(reason.contains("15.0%"));
            }
            RiskDecision::Allow => panic!("Should block high slippage"),
        }
    }
}