/// Liquidity Change Detector
/// 
/// Detects significant liquidity changes and scams in pools including:
/// - Pool liquidity drains (>60% drain OR <0.3 ETH remaining)
/// - Major/significant liquidity removals
/// - Complete pool drains indicating potential scams

use std::collections::HashMap;
use tracing::info;
use alloy_primitives::{Address, I256};
use reth_tx_simulator::AddressStateChange;
use crate::common::address::alloy_address_to_checksum;
use crate::token_tracking::TokenTrackingCache;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct LiquiditySignal {
    pub signal_type: SignalType,
    pub pool_address: String,
    pub token_address: String,
    pub pool_type: String,  // Added pool type (V2, V3, V4)
    pub change_type: LiquidityChangeType,
    pub eth_change: f64,
    pub percentage_change: f64,
    pub remaining_liquidity: f64,
    pub from_address: String,
    pub tx_hash: String,
    pub details: String,
    // Additional fields for database
    pub eth_removed: Option<f64>,
    pub token_removed: Option<f64>,
    pub remaining_eth: Option<f64>,
    pub remaining_token: Option<f64>,
    pub removal_percentage: Option<f64>,
    pub creator_address: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SignalType {
    LiquidityRemoval,
    ScamDetected,
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
    /// Complete drain (>60% or <0.3 ETH) - SCAM
    CompleteDrain,
}

/// Drain calculation result
#[derive(Debug, Clone)]
struct DrainResult {
    pub current_reserve: f64,
    pub eth_change: f64,
    pub new_reserve: f64,
    pub drain_percent: f64,
}

pub struct LiquidityDetector {
    /// Threshold for scam drain detection (60%)
    scam_drain_threshold: f64,
    /// Minimum ETH to not consider drained
    min_eth_threshold: f64,
    /// Threshold for major removal
    major_removal_threshold: f64,
    /// Threshold for significant removal
    significant_removal_threshold: f64,
    /// Token tracking cache for pool information
    token_cache: Option<Arc<TokenTrackingCache>>,
}

impl Default for LiquidityDetector {
    fn default() -> Self {
        Self {
            scam_drain_threshold: 0.6,      // 60% for scam detection
            min_eth_threshold: 0.3,         // 0.3 ETH
            major_removal_threshold: 0.5,   // 50%
            significant_removal_threshold: 0.2, // 20%
            token_cache: None,
        }
    }
}

impl LiquidityDetector {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the token tracking cache
    pub fn set_token_cache(&mut self, token_cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(token_cache);
        info!("📊 Token tracking cache connected to liquidity detector");
    }

    /// Detect liquidity changes and scams from state changes
    pub async fn detect(
        &self,
        tx_hash: &str,
        from_address: Address,
        state_changes: &HashMap<Address, AddressStateChange>,
    ) -> Vec<LiquiditySignal> {
        let mut signals = Vec::new();

        // Check each address for liquidity changes or scams
        for (address, changes) in state_changes {
            if let Some(signal) = self.check_address_for_drain(
                address,
                changes,
                tx_hash,
                from_address,
            ).await {
                signals.push(signal);
            }
        }

        signals
    }

    /// Check if a specific address shows liquidity drain or scam activity
    async fn check_address_for_drain(
        &self,
        address: &Address,
        changes: &AddressStateChange,
        tx_hash: &str,
        from_address: Address,
    ) -> Option<LiquiditySignal> {
        let address_str = alloy_address_to_checksum(*address);
        
        // Step 1: Check if this address is a tracked pool
        let pool_state = if let Some(ref token_cache) = self.token_cache {
            token_cache.get_pool(&address_str).await?
        } else {
            // Without token cache, we can't determine if this is a pool
            return None;
        };
        
        // Skip pools with very low initial liquidity (< 0.1 ETH)
        // These are likely already dead pools, not worth monitoring
        if pool_state.eth_reserve < 0.1 {
            return None;
        }
        
        // Step 2: Check if there's negative ETH change (drain)
        let eth_change = changes.eth_net;
        if eth_change >= I256::ZERO {
            return None; // No drain
        }
        
        // Step 3: Calculate drain metrics
        let eth_change_f64 = eth_change.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        let drain_result = self.calculate_drain_metrics(&pool_state, eth_change_f64);
        
        // Step 4: Determine signal type and change type
        let (signal_type, change_type) = if self.is_scam_drain(&drain_result) {
            (SignalType::ScamDetected, LiquidityChangeType::CompleteDrain)
        } else if drain_result.drain_percent > self.major_removal_threshold * 100.0 {
            (SignalType::LiquidityRemoval, LiquidityChangeType::MajorRemoval)
        } else if drain_result.drain_percent > self.significant_removal_threshold * 100.0 {
            (SignalType::LiquidityRemoval, LiquidityChangeType::SignificantRemoval)
        } else {
            return None; // Minor change, ignore
        };
        
        let details = match signal_type {
            SignalType::ScamDetected => 
                format!("SCAM DETECTED - Pool drain: {:.1}% ({:.6} ETH -> {:.6} ETH)", 
                    drain_result.drain_percent, drain_result.current_reserve, drain_result.new_reserve),
            SignalType::LiquidityRemoval => 
                format!("Liquidity removal: {:.2} ETH ({:.1}%)", 
                    eth_change_f64.abs(), drain_result.drain_percent),
        };
        
        info!("💧 {} for pool {}", details, address_str);
        
        Some(LiquiditySignal {
            signal_type,
            pool_address: address_str,
            token_address: pool_state.token_address.clone(),
            pool_type: format!("{:?}", pool_state.pool_type),  // Convert enum to string
            change_type,
            eth_change: eth_change_f64,
            percentage_change: drain_result.drain_percent,
            remaining_liquidity: drain_result.new_reserve,
            from_address: alloy_address_to_checksum(from_address),
            tx_hash: tx_hash.to_string(),
            details,
            // Additional fields for database
            eth_removed: Some(eth_change_f64.abs()),
            token_removed: None, // TODO: Calculate from state changes
            remaining_eth: Some(drain_result.new_reserve),
            remaining_token: None, // TODO: Calculate from state changes
            removal_percentage: Some(drain_result.drain_percent),
            creator_address: alloy_address_to_checksum(from_address), // Use from_address as creator for now
        })
    }
    
    /// Calculate drain metrics for a pool
    fn calculate_drain_metrics(&self, pool_state: &crate::token_tracking::types::Pool, eth_change: f64) -> DrainResult {
        let current_reserve = pool_state.eth_reserve;
        
        // If pool already has 0 or very low reserves, skip percentage calculation
        if current_reserve < 0.001 {
            return DrainResult {
                current_reserve,
                eth_change,
                new_reserve: 0.0,
                drain_percent: 0.0,
            };
        }
        
        // Calculate new reserve (eth_change is negative for drains)
        let new_reserve = (current_reserve + eth_change).max(0.0);
        let drain_percent = ((current_reserve - new_reserve) / current_reserve) * 100.0;
        
        DrainResult {
            current_reserve,
            eth_change,
            new_reserve,
            drain_percent,
        }
    }
    
    /// Check if drain is significant enough to be a scam (>60% drain OR <=0.3 ETH remaining)
    fn is_scam_drain(&self, drain_result: &DrainResult) -> bool {
        drain_result.drain_percent > self.scam_drain_threshold * 100.0 || 
        drain_result.new_reserve <= self.min_eth_threshold
    }
    
}