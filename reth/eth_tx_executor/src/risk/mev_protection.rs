//! MEV Protection Module
//!
//! Ensures our protective transactions execute before detected scam transactions
//! by implementing various frontrunning and priority strategies

use ethers::types::{U256, H256};
use std::collections::HashMap;
use tracing::{info, warn, debug};
use crate::alert_processor::Alert;

/// MEV protection strategies
#[derive(Debug, Clone)]
pub enum MEVStrategy {
    /// Simple gas price multiplier (e.g., 1.5x detected tx gas)
    GasMultiplier(f64),
    /// Fixed high priority fee
    FixedPriority(U256),
    /// Adaptive based on network conditions
    Adaptive { base_fee: U256, priority_multiplier: f64 },
    /// Flashbots bundle submission (advanced)
    FlashbotsBundle,
}

/// Priority configuration for transaction ordering
#[derive(Debug, Clone)]
pub struct PriorityConfig {
    /// Strategy to use for MEV protection
    pub strategy: MEVStrategy,
    /// Maximum gas price we're willing to pay (safety limit)
    pub max_gas_price: U256,
    /// Emergency multiplier for critical alerts
    pub emergency_multiplier: f64,
    /// Enable bundle submission for guaranteed ordering
    pub use_bundles: bool,
}

impl Default for PriorityConfig {
    fn default() -> Self {
        Self {
            strategy: MEVStrategy::GasMultiplier(1.5), // 150% of detected tx
            max_gas_price: U256::from(100_000_000_000u64), // 100 gwei max
            emergency_multiplier: 2.0, // 200% for emergency
            use_bundles: false, // Disabled by default
        }
    }
}

/// MEV protector handles transaction prioritization
pub struct MEVProtector {
    config: PriorityConfig,
    /// Track gas prices of detected transactions
    detected_tx_gas_prices: HashMap<H256, U256>,
    /// Current network base fee estimate
    current_base_fee: U256,
}

impl MEVProtector {
    pub fn new(config: PriorityConfig) -> Self {
        Self {
            config,
            detected_tx_gas_prices: HashMap::new(),
            current_base_fee: U256::from(20_000_000_000u64), // 20 gwei default
        }
    }
    
    /// Calculate optimal gas price to frontrun a detected scam transaction
    pub fn calculate_frontrun_gas_price(
        &mut self,
        alert: &Alert,
        is_emergency: bool,
    ) -> Result<(U256, U256), Box<dyn std::error::Error>> {
        // Extract gas price from the detected transaction
        let detected_gas_price = self.extract_gas_price_from_alert(alert)?;
        
        // Store for tracking (using hash of alert ID)
        let alert_hash = H256::from(ethers::utils::keccak256(alert.id.as_bytes()));
        self.detected_tx_gas_prices.insert(alert_hash, detected_gas_price);
        
        // Calculate our gas price based on strategy
        let (base_fee, priority_fee) = match &self.config.strategy {
            MEVStrategy::GasMultiplier(multiplier) => {
                let multiplier = if is_emergency {
                    multiplier * self.config.emergency_multiplier
                } else {
                    *multiplier
                };
                
                let our_gas_price = U256::from((detected_gas_price.as_u128() as f64 * multiplier) as u128);
                
                info!(
                    "MEV Protection: Detected gas price: {} gwei, our price: {} gwei ({}x multiplier)",
                    detected_gas_price / 1_000_000_000,
                    our_gas_price / 1_000_000_000,
                    multiplier
                );
                
                // For EIP-1559, split into base fee + priority
                let base_fee = std::cmp::min(our_gas_price / 2, self.current_base_fee);
                let priority_fee = our_gas_price.saturating_sub(base_fee);
                
                (base_fee, priority_fee)
            }
            
            MEVStrategy::FixedPriority(priority) => {
                let priority = if is_emergency {
                    *priority * 2
                } else {
                    *priority
                };
                
                (self.current_base_fee, priority)
            }
            
            MEVStrategy::Adaptive { base_fee, priority_multiplier } => {
                let priority = U256::from((detected_gas_price.as_u128() as f64 * priority_multiplier) as u128);
                (*base_fee, priority)
            }
            
            MEVStrategy::FlashbotsBundle => {
                // For bundles, we can use lower gas but guarantee ordering
                let priority = detected_gas_price / 2; // 50% of detected tx
                (self.current_base_fee, priority)
            }
        };
        
        // Apply safety limits
        let max_total = self.config.max_gas_price;
        let total_gas = base_fee + priority_fee;
        
        if total_gas > max_total {
            warn!(
                "Calculated gas price {} gwei exceeds max {} gwei, capping",
                total_gas / 1_000_000_000,
                max_total / 1_000_000_000
            );
            
            let capped_priority = max_total.saturating_sub(base_fee);
            return Ok((base_fee, capped_priority));
        }
        
        Ok((base_fee, priority_fee))
    }
    
    /// Extract gas price from alert
    fn extract_gas_price_from_alert(&self, alert: &Alert) -> Result<U256, Box<dyn std::error::Error>> {
        // Use max gas price if provided in alert
        if let Some(max_gas) = alert.params.max_gas_price {
            return Ok(max_gas);
        }
        
        // Fallback: estimate based on network conditions
        warn!(
            "No gas price in alert {}, using network estimate",
            alert.id
        );
        
        // Use current base fee + reasonable priority
        let estimated_gas = self.current_base_fee + U256::from(2_000_000_000u64); // +2 gwei priority
        Ok(estimated_gas)
    }
    
    /// Update current network base fee (should be called periodically)
    pub fn update_base_fee(&mut self, base_fee: U256) {
        if base_fee != self.current_base_fee {
            debug!("Updated base fee: {} gwei", base_fee / 1_000_000_000);
            self.current_base_fee = base_fee;
        }
    }
    
    /// Check if we should use Flashbots bundles for this alert
    pub fn should_use_bundle(&self, alert: &Alert) -> bool {
        if !self.config.use_bundles {
            return false;
        }
        
        // Use bundles for critical priority alerts
        matches!(alert.params.priority, crate::alert_processor::Priority::Critical)
    }
    
    /// Calculate time advantage needed to frontrun
    pub fn calculate_time_advantage(&self, alert: &Alert) -> std::time::Duration {
        // For critical alerts, assume we need ultra-low latency
        let detected_latency_ms = match alert.params.priority {
            crate::alert_processor::Priority::Critical => 10,
            crate::alert_processor::Priority::High => 50,
            crate::alert_processor::Priority::Normal => 100,
        };
        
        // We need to submit our tx in less time than it took to detect the scam
        let target_latency_ms = std::cmp::max(detected_latency_ms / 2, 10); // At least 10ms
        
        std::time::Duration::from_millis(target_latency_ms)
    }
    
    /// Get statistics on MEV protection effectiveness
    pub fn get_mev_stats(&self) -> MEVStats {
        MEVStats {
            total_frontrun_attempts: self.detected_tx_gas_prices.len(),
            average_gas_multiplier: self.calculate_average_multiplier(),
            current_base_fee: self.current_base_fee,
        }
    }
    
    fn calculate_average_multiplier(&self) -> f64 {
        if self.detected_tx_gas_prices.is_empty() {
            return 1.0;
        }
        
        // This would need actual execution data to calculate properly
        1.5 // Placeholder
    }
}

/// Statistics about MEV protection effectiveness
#[derive(Debug, Clone)]
pub struct MEVStats {
    pub total_frontrun_attempts: usize,
    pub average_gas_multiplier: f64,
    pub current_base_fee: U256,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alert_processor::{Action, ExecutionParams, Priority};
    
    fn create_test_alert(gas_price_gwei: f64) -> Alert {
        Alert {
            id: "test_mev".to_string(),
            timestamp: 1234567890,
            token_address: Address::zero(),
            pool_address: Address::zero(),
            action: crate::alert_processor::Action::Sell,
            params: crate::alert_processor::ExecutionParams {
                amount: U256::from(1000),
                slippage: 0.05,
                max_gas_price: Some(U256::from((gas_price_gwei * 1e9) as u64)),
                deadline_seconds: 300,
                priority: crate::alert_processor::Priority::Normal,
            },
        }
    }
    
    #[test]
    fn test_gas_multiplier_strategy() {
        let config = PriorityConfig {
            strategy: MEVStrategy::GasMultiplier(1.5),
            ..Default::default()
        };
        
        let mut protector = MEVProtector::new(config);
        let alert = create_test_alert(30.0); // 30 gwei detected
        
        let (base_fee, priority_fee) = protector
            .calculate_frontrun_gas_price(&alert, false)
            .unwrap();
        
        let total = base_fee + priority_fee;
        let expected = U256::from(45_000_000_000u64); // 45 gwei (30 * 1.5)
        
        assert!(total >= expected * 90 / 100); // Within 10% tolerance
        assert!(total <= expected * 110 / 100);
    }
    
    #[test]
    fn test_emergency_multiplier() {
        let config = PriorityConfig {
            strategy: MEVStrategy::GasMultiplier(1.5),
            emergency_multiplier: 2.0,
            ..Default::default()
        };
        
        let mut protector = MEVProtector::new(config);
        let alert = create_test_alert(20.0); // 20 gwei detected
        
        let (base_fee, priority_fee) = protector
            .calculate_frontrun_gas_price(&alert, true) // Emergency
            .unwrap();
        
        let total = base_fee + priority_fee;
        let expected = U256::from(60_000_000_000u64); // 60 gwei (20 * 1.5 * 2.0)
        
        assert!(total >= expected * 90 / 100);
        assert!(total <= expected * 110 / 100);
    }
    
    #[test]
    fn test_gas_price_cap() {
        let config = PriorityConfig {
            strategy: MEVStrategy::GasMultiplier(10.0), // Very high multiplier
            max_gas_price: U256::from(50_000_000_000u64), // 50 gwei cap
            ..Default::default()
        };
        
        let mut protector = MEVProtector::new(config);
        let alert = create_test_alert(100.0); // 100 gwei detected
        
        let (base_fee, priority_fee) = protector
            .calculate_frontrun_gas_price(&alert, false)
            .unwrap();
        
        let total = base_fee + priority_fee;
        assert!(total <= U256::from(50_000_000_000u64)); // Should be capped
    }
    
    #[test]
    fn test_time_advantage_calculation() {
        let config = PriorityConfig::default();
        let protector = MEVProtector::new(config);
        
        let alert = create_test_alert(30.0);
        let time_advantage = protector.calculate_time_advantage(&alert);
        
        // Should be half of detected latency, but at least 10ms
        assert!(time_advantage >= std::time::Duration::from_millis(10));
        assert!(time_advantage <= std::time::Duration::from_millis(5000));
    }
}