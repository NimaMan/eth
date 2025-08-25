/// ETF flow analysis
/// 
/// Analyzes inflows and outflows for ETF providers.

use crate::{ChainQuery, Result};
use crate::entities::common::{FlowDirection, BalanceChange, format_token_amount};
use super::addresses::{is_etf_address, get_etf_by_address, get_provider_addresses, provider_stats};
use alloy_primitives::{Address, U256};
use std::sync::Arc;
use std::collections::HashMap;

/// Flow summary for an ETF provider
#[derive(Debug, Clone)]
pub struct ProviderFlow {
    pub provider_name: &'static str,
    pub balance_change: BalanceChange,
    pub flow_direction: FlowDirection,
}

/// Flow data for a provider between blocks
#[derive(Debug, Clone)]
pub struct FlowData {
    pub provider: String,
    pub inflow_eth: f64,
    pub outflow_eth: f64,
    pub net_flow_eth: f64,
    pub transfer_count: u32,
    pub unique_addresses: u32,
}

/// Aggregated flow data for all providers
#[derive(Debug, Clone)]
pub struct ETFFlowSummary {
    pub from_block: u64,
    pub to_block: u64,
    pub provider_flows: HashMap<String, FlowData>,
    pub total_inflow_eth: f64,
    pub total_outflow_eth: f64,
    pub total_net_flow_eth: f64,
}

/// ETF flow analyzer
pub struct EtfFlowAnalyzer {
    chain_query: Arc<ChainQuery>,
}

impl EtfFlowAnalyzer {
    /// Create new flow analyzer
    pub fn new(chain_query: Arc<ChainQuery>) -> Self {
        Self { chain_query }
    }
    
    /// Analyze flow direction for a transfer
    pub fn analyze_transfer_flow(&self, from: Address, to: Address) -> Option<(&'static str, FlowDirection)> {
        let from_etf = get_etf_by_address(from);
        let to_etf = get_etf_by_address(to);
        
        match (from_etf, to_etf) {
            (Some(from_info), Some(to_info)) => {
                if from_info.provider == to_info.provider {
                    // Internal transfer within same provider
                    Some((from_info.provider, FlowDirection::Internal))
                } else {
                    // Inter-provider transfer (rare but possible)
                    Some((from_info.provider, FlowDirection::Outflow))
                }
            }
            (Some(from_info), None) => {
                // Outflow from ETF (redemption or transfer out)
                Some((from_info.provider, FlowDirection::Outflow))
            }
            (None, Some(to_info)) => {
                // Inflow to ETF (creation or transfer in)
                Some((to_info.provider, FlowDirection::Inflow))
            }
            (None, None) => None,
        }
    }
    
    /// Calculate balance changes for a provider between two blocks
    pub async fn get_provider_balance_change(
        &self,
        provider_name: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<Option<BalanceChange>> {
        if let Some(addresses) = get_provider_addresses(provider_name) {
            let mut old_balance = U256::ZERO;
            let mut new_balance = U256::ZERO;
            
            for address in addresses {
                let old = self.chain_query.account
                    .get_balance(*address, Some(from_block))
                    .await
                    .unwrap_or(U256::ZERO);
                let new = self.chain_query.account
                    .get_balance(*address, Some(to_block))
                    .await
                    .unwrap_or(U256::ZERO);
                
                old_balance = old_balance.saturating_add(old);
                new_balance = new_balance.saturating_add(new);
            }
            
            // Use first address as representative for the provider
            let representative_address = addresses.first().copied().unwrap_or(Address::ZERO);
            
            Ok(Some(BalanceChange::calculate(
                representative_address,
                old_balance,
                new_balance,
            )))
        } else {
            Ok(None)
        }
    }
    
    /// Check if transfer involves any ETF
    pub fn involves_etf(&self, from: Address, to: Address) -> bool {
        is_etf_address(from) || is_etf_address(to)
    }
    
    /// Identify creation events (large inflows to ETF)
    pub fn is_likely_creation(&self, to: Address, amount: U256) -> bool {
        if is_etf_address(to) {
            // Consider creation if > 1000 ETH
            let threshold = U256::from(1000) * U256::from(10).pow(U256::from(18));
            amount > threshold
        } else {
            false
        }
    }
    
    /// Identify redemption events (large outflows from ETF)
    pub fn is_likely_redemption(&self, from: Address, amount: U256) -> bool {
        if is_etf_address(from) {
            // Consider redemption if > 1000 ETH
            let threshold = U256::from(1000) * U256::from(10).pow(U256::from(18));
            amount > threshold
        } else {
            false
        }
    }
    
    /// Get net flow for all ETF providers between two blocks
    pub async fn get_total_etf_net_flow(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<i128> {
        let mut total_old_balance = U256::ZERO;
        let mut total_new_balance = U256::ZERO;
        
        // Sum across all providers
        for (provider_name, _) in super::addresses::provider_stats() {
            if let Some(addresses) = get_provider_addresses(provider_name) {
                for address in addresses {
                    let old = self.chain_query.account
                        .get_balance(*address, Some(from_block))
                        .await
                        .unwrap_or(U256::ZERO);
                    let new = self.chain_query.account
                        .get_balance(*address, Some(to_block))
                        .await
                        .unwrap_or(U256::ZERO);
                    
                    total_old_balance = total_old_balance.saturating_add(old);
                    total_new_balance = total_new_balance.saturating_add(new);
                }
            }
        }
        
        let net_flow = if total_new_balance >= total_old_balance {
            (total_new_balance - total_old_balance).to_string().parse::<i128>().unwrap_or(i128::MAX)
        } else {
            -(total_old_balance - total_new_balance).to_string().parse::<i128>().unwrap_or(i128::MIN)
        };
        
        Ok(net_flow)
    }
    
    /// Calculate flows for all ETF providers between two blocks
    pub async fn calculate_flows_between_blocks(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<ETFFlowSummary> {
        let mut provider_flows = HashMap::new();
        let mut total_inflow = 0.0;
        let mut total_outflow = 0.0;
        
        // Calculate flow for each provider
        for (provider_name, _) in provider_stats() {
            if let Some(addresses) = get_provider_addresses(provider_name) {
                let mut provider_old_balance = U256::ZERO;
                let mut provider_new_balance = U256::ZERO;
                let mut unique_addrs = std::collections::HashSet::new();
                
                // Sum balances across all addresses for this provider
                for address in addresses {
                    // Get old balance
                    let old = self.chain_query.account
                        .get_balance(*address, Some(from_block))
                        .await
                        .unwrap_or(U256::ZERO);
                    provider_old_balance = provider_old_balance.saturating_add(old);
                    
                    // Get new balance
                    let new = self.chain_query.account
                        .get_balance(*address, Some(to_block))
                        .await
                        .unwrap_or(U256::ZERO);
                    provider_new_balance = provider_new_balance.saturating_add(new);
                    
                    // Track unique addresses (simplified - in production would check actual transfers)
                    unique_addrs.insert(*address);
                }
                
                // Calculate flow
                let old_eth = format_token_amount(provider_old_balance, 18);
                let new_eth = format_token_amount(provider_new_balance, 18);
                
                let (inflow, outflow) = if new_eth > old_eth {
                    // Net inflow
                    (new_eth - old_eth, 0.0)
                } else if old_eth > new_eth {
                    // Net outflow
                    (0.0, old_eth - new_eth)
                } else {
                    // No change
                    (0.0, 0.0)
                };
                
                let net_flow = inflow - outflow;
                
                // Only include if there was activity
                if inflow > 0.0 || outflow > 0.0 {
                    total_inflow += inflow;
                    total_outflow += outflow;
                    
                    provider_flows.insert(
                        provider_name.to_string(),
                        FlowData {
                            provider: provider_name.to_string(),
                            inflow_eth: inflow,
                            outflow_eth: outflow,
                            net_flow_eth: net_flow,
                            transfer_count: if net_flow != 0.0 { 1 } else { 0 }, // Simplified
                            unique_addresses: unique_addrs.len() as u32,
                        }
                    );
                }
            }
        }
        
        Ok(ETFFlowSummary {
            from_block,
            to_block,
            provider_flows,
            total_inflow_eth: total_inflow,
            total_outflow_eth: total_outflow,
            total_net_flow_eth: total_inflow - total_outflow,
        })
    }
}