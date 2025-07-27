/// Liquidity Change Detector
/// 
/// Detects significant liquidity changes in pools

use crate::signal_engine::simulator::{SimulationResult, StateChange};
use tracing::{info, debug};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LiquiditySignal {
    pub pool_address: String,
    pub token_address: String,
    pub change_type: LiquidityChangeType,
    pub eth_change: f64,
    pub percentage_change: f64,
    pub remaining_liquidity: f64,
    pub confidence: f64,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiquidityChangeType {
    /// Major liquidity removal (>50%)
    MajorRemoval,
    /// Significant removal (20-50%)
    SignificantRemoval,
    /// Minor removal (<20%)
    MinorRemoval,
    /// Liquidity addition
    Addition,
    /// Complete drain (>90% or <0.3 ETH)
    CompleteDrain,
}

pub struct LiquidityDetector {
    /// Threshold for complete drain detection
    drain_threshold: f64,
    /// Minimum ETH to not consider drained
    min_eth_threshold: f64,
    /// Threshold for major removal
    major_removal_threshold: f64,
    /// Threshold for significant removal
    significant_removal_threshold: f64,
}

impl Default for LiquidityDetector {
    fn default() -> Self {
        Self {
            drain_threshold: 0.9,          // 90%
            min_eth_threshold: 0.3,         // 0.3 ETH
            major_removal_threshold: 0.5,   // 50%
            significant_removal_threshold: 0.2, // 20%
        }
    }
}

impl LiquidityDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect liquidity changes from simulation
    pub fn detect(&self, sim_result: &SimulationResult) -> Vec<LiquiditySignal> {
        let mut signals = Vec::new();

        // Get state changes from transaction simulation
        if let Some(ref tx_sim) = sim_result.tx_simulation {
            for (address, state_change) in &tx_sim.state_changes {
                if let Some(signal) = self.analyze_state_change(address, state_change, sim_result) {
                    signals.push(signal);
                }
            }
        }

        signals
    }

    fn analyze_state_change(
        &self,
        pool_address: &str,
        state_change: &StateChange,
        sim_result: &SimulationResult,
    ) -> Option<LiquiditySignal> {
        // Skip if no ETH change
        if state_change.eth_change.abs() < 0.01 {
            return None;
        }

        // For liquidity detection, we need negative ETH change (removal)
        let eth_change = state_change.eth_change;
        
        // Get current liquidity (would come from pool cache in real implementation)
        let current_liquidity = 10.0; // Placeholder
        let remaining_liquidity = current_liquidity + eth_change;
        let percentage_change = (eth_change.abs() / current_liquidity) * 100.0;

        // Determine change type
        let change_type = if eth_change > 0.0 {
            LiquidityChangeType::Addition
        } else if remaining_liquidity < self.min_eth_threshold || percentage_change > self.drain_threshold * 100.0 {
            LiquidityChangeType::CompleteDrain
        } else if percentage_change > self.major_removal_threshold * 100.0 {
            LiquidityChangeType::MajorRemoval
        } else if percentage_change > self.significant_removal_threshold * 100.0 {
            LiquidityChangeType::SignificantRemoval
        } else {
            LiquidityChangeType::MinorRemoval
        };

        // Only signal significant changes
        if matches!(change_type, LiquidityChangeType::MinorRemoval | LiquidityChangeType::Addition) 
            && percentage_change < 10.0 {
            return None;
        }

        // Extract token address
        let token_address = match &sim_result.request.category {
            crate::signal_engine::classifier::TransactionCategory::DexInteraction { token_address, .. } => {
                token_address.as_ref()?.clone()
            }
            _ => "unknown".to_string(),
        };

        let confidence = match change_type {
            LiquidityChangeType::CompleteDrain => 0.95,
            LiquidityChangeType::MajorRemoval => 0.9,
            LiquidityChangeType::SignificantRemoval => 0.8,
            _ => 0.7,
        };

        let details = match change_type {
            LiquidityChangeType::CompleteDrain => 
                format!("Pool drained: {:.2} ETH removed ({:.1}%), {:.2} ETH remaining", 
                    eth_change.abs(), percentage_change, remaining_liquidity),
            LiquidityChangeType::MajorRemoval =>
                format!("Major liquidity removal: {:.2} ETH ({:.1}%)", 
                    eth_change.abs(), percentage_change),
            LiquidityChangeType::SignificantRemoval =>
                format!("Significant liquidity removal: {:.2} ETH ({:.1}%)", 
                    eth_change.abs(), percentage_change),
            LiquidityChangeType::Addition =>
                format!("Liquidity added: {:.2} ETH ({:.1}%)", 
                    eth_change.abs(), percentage_change),
            _ => format!("Liquidity change: {:.2} ETH", eth_change),
        };

        info!("💧 {} for pool {}", details, pool_address);

        Some(LiquiditySignal {
            pool_address: pool_address.to_string(),
            token_address,
            change_type,
            eth_change,
            percentage_change,
            remaining_liquidity,
            confidence,
            details,
        })
    }
}