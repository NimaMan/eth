// token_tracking/signal_integration.rs
//
// Integration between token tracking cache and signal detection

use super::address_tracking_cache::{AddressTrackingCache, AddressRole};
// These would be imported from the actual signal engine when integrated
#[derive(Debug, Clone)]
pub struct DetectedFunction {
    pub selector: [u8; 4],
    pub name: String,
    pub signature: String,
}

#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub state_changes: Vec<StateChange>,
}

#[derive(Debug, Clone)]
pub struct StateChange {
    pub address: String,
    pub balance_change: Option<f64>,
}
use tracing::warn;

/// Critical functions that trigger immediate signals
pub const CRITICAL_FUNCTIONS: &[&str] = &[
    "removeLiquidity",
    "removeLiquidityETH",
    "removeLiquidityWithPermit",
    "removeLiquidityETHWithPermit",
    "removeLiquidityETHSupportingFeeOnTransferTokens",
    "removeLiquidityETHWithPermitSupportingFeeOnTransferTokens",
    "transferOwnership",
    "renounceOwnership",
    "pause",
    "unpause",
    "blacklist",
    "setMaxTxAmount",
    "setMaxWalletSize",
    "enableTrading",
    "openTrading",
];

/// Pool drain threshold (percentage)
const POOL_DRAIN_THRESHOLD: f64 = 0.5; // 50% drain

/// Minimum ETH in pool after transaction
const MIN_ETH_THRESHOLD: f64 = 0.3; // 0.3 ETH

#[derive(Debug, Clone)]
pub struct CreatorActionSignal {
    pub token_address: String,
    pub creator_address: String,
    pub function_name: String,
    pub role: AddressRole,
    pub tx_hash: String,
    pub timestamp: u64,
    pub severity: SignalSeverity,
}

#[derive(Debug, Clone)]
pub struct PoolDrainSignal {
    pub token_address: String,
    pub pool_address: String,
    pub eth_before: f64,
    pub eth_after: f64,
    pub drain_percentage: f64,
    pub tx_hash: String,
    pub from_address: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SignalSeverity {
    Low,
    Medium,
    High,
    Critical,
}

pub struct SignalIntegration {
    cache: AddressTrackingCache,
}

impl SignalIntegration {
    pub fn new(cache: AddressTrackingCache) -> Self {
        Self { cache }
    }
    
    /// Process a mempool transaction for signal detection
    pub async fn process_mempool_transaction(
        &self,
        from_address: &str,
        tx_hash: &str,
        detected_functions: &[DetectedFunction],
        timestamp: u64,
    ) -> Vec<CreatorActionSignal> {
        let mut signals = Vec::new();
        
        // Check if this address is tracked
        if let Some(address_info) = self.cache.get_address_info(from_address).await {
            // Process each detected function
            for func in detected_functions {
                // Record the function call
                self.cache.record_function_call(
                    from_address,
                    func.selector,
                    func.name.clone(),
                    tx_hash.to_string(),
                    timestamp,
                ).await;
                
                // Check if it's a critical function
                if CRITICAL_FUNCTIONS.contains(&func.name.as_str()) {
                    // Generate signal for each token this address is associated with
                    for (token_address, role) in &address_info.tokens {
                        let severity = determine_severity(&func.name, role, address_info.is_high_risk);
                        
                        let signal = CreatorActionSignal {
                            token_address: token_address.clone(),
                            creator_address: from_address.to_string(),
                            function_name: func.name.clone(),
                            role: role.clone(),
                            tx_hash: tx_hash.to_string(),
                            timestamp,
                            severity,
                        };
                        
                        warn!(
                            "CRITICAL: {} ({:?}) calling {} on token {}",
                            from_address, role, func.name, token_address
                        );
                        
                        signals.push(signal);
                    }
                }
            }
        }
        
        signals
    }
    
    /// Process simulation results for pool drain detection
    pub async fn process_simulation_results(
        &self,
        simulation: &SimulationResult,
        tx_hash: &str,
        from_address: &str,
    ) -> Vec<PoolDrainSignal> {
        let mut signals = Vec::new();
        
        // Check each state change
        for state_change in &simulation.state_changes {
            // Check if this is a pool we're tracking
            if let Some((token_address, pool_info)) = self.cache.get_pool_info(&state_change.address).await {
                // Look for ETH balance changes
                if let Some(balance_change) = state_change.balance_change {
                    let eth_before = pool_info.current_eth_reserve;
                    let eth_after = eth_before + balance_change;
                    
                    // Check for significant drain
                    if eth_after < eth_before {
                        let drain_percentage = (eth_before - eth_after) / eth_before;
                        
                        if drain_percentage >= POOL_DRAIN_THRESHOLD || eth_after < MIN_ETH_THRESHOLD {
                            let signal = PoolDrainSignal {
                                token_address: token_address.clone(),
                                pool_address: state_change.address.clone(),
                                eth_before,
                                eth_after,
                                drain_percentage: drain_percentage * 100.0,
                                tx_hash: tx_hash.to_string(),
                                from_address: from_address.to_string(),
                            };
                            
                            warn!(
                                "POOL DRAIN: {} drained {:.1}% from pool {} (token: {})",
                                from_address, drain_percentage * 100.0, state_change.address, token_address
                            );
                            
                            signals.push(signal);
                        }
                    }
                }
            }
        }
        
        signals
    }
    
    /// Check if we should simulate this transaction
    pub async fn should_simulate_transaction(
        &self,
        from_address: &str,
        detected_functions: &[DetectedFunction],
    ) -> bool {
        // Always simulate if from a tracked creator/owner
        if self.cache.get_address_info(from_address).await.is_some() {
            return true;
        }
        
        // Simulate if transaction has interesting functions
        for func in detected_functions {
            if CRITICAL_FUNCTIONS.contains(&func.name.as_str()) {
                return true;
            }
        }
        
        false
    }
}

fn determine_severity(function_name: &str, role: &AddressRole, is_high_risk: bool) -> SignalSeverity {
    if is_high_risk {
        return SignalSeverity::Critical;
    }
    
    match function_name {
        "removeLiquidity" | "removeLiquidityETH" | "removeLiquidityWithPermit" => {
            match role {
                AddressRole::Creator | AddressRole::Both => SignalSeverity::Critical,
                AddressRole::Owner => SignalSeverity::High,
            }
        }
        "transferOwnership" | "renounceOwnership" => SignalSeverity::High,
        "pause" | "blacklist" => SignalSeverity::High,
        "enableTrading" | "openTrading" => SignalSeverity::Medium,
        _ => SignalSeverity::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_signal_detection() {
        let cache = AddressTrackingCache::new();
        let integration = SignalIntegration::new(cache.clone());
        
        // Add test token data
        cache.update_token_data(
            "0xtoken123",
            "0xcreator456",
            "0xcreator456", // Same as creator
            vec![("0xpool789".to_string(), 10.0, 100000.0)],
            true,
            false,
        ).await;
        
        // Test function detection
        let functions = vec![
            DetectedFunction {
                selector: [0x12, 0x34, 0x56, 0x78],
                name: "removeLiquidity".to_string(),
                signature: "removeLiquidity(uint256,uint256)".to_string(),
            }
        ];
        
        let signals = integration.process_mempool_transaction(
            "0xcreator456",
            "0xtxhash",
            &functions,
            1234567890,
        ).await;
        
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].function_name, "removeLiquidity");
        assert_eq!(signals[0].severity, SignalSeverity::Critical);
    }
}