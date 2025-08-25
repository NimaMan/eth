/// CEX flow analysis
/// 
/// Analyzes deposits and withdrawals to/from CEX addresses.

use crate::{ChainQuery, Result};
use crate::entities::common::{FlowDirection, categorize_flow, format_token_amount};
use super::addresses::{is_cex_address, get_cex_by_address, get_exchange_addresses, exchange_stats};
use alloy_primitives::{Address, U256};
use std::sync::Arc;
use std::collections::HashMap;

/// Flow summary for an exchange
#[derive(Debug, Clone)]
pub struct ExchangeFlow {
    pub exchange_name: &'static str,
    pub total_inflow: U256,
    pub total_outflow: U256,
    pub net_flow: i128, // Can be negative
    pub inflow_count: u64,
    pub outflow_count: u64,
}

/// Flow data for an exchange between blocks
#[derive(Debug, Clone)]
pub struct CexFlowData {
    pub exchange: String,
    pub inflow_eth: f64,
    pub outflow_eth: f64,
    pub net_flow_eth: f64,
    pub transfer_count: u32,
    pub unique_addresses: u32,
}

/// Aggregated flow data for all exchanges
#[derive(Debug, Clone)]
pub struct CexFlowSummary {
    pub from_block: u64,
    pub to_block: u64,
    pub exchange_flows: HashMap<String, CexFlowData>,
    pub total_inflow_eth: f64,
    pub total_outflow_eth: f64,
    pub total_net_flow_eth: f64,
}

/// CEX flow analyzer
pub struct CexFlowAnalyzer {
    chain_query: Arc<ChainQuery>,
}

impl CexFlowAnalyzer {
    /// Create new flow analyzer
    pub fn new(chain_query: Arc<ChainQuery>) -> Self {
        Self { chain_query }
    }
    
    /// Analyze flow direction for a transfer
    pub fn analyze_transfer_flow(&self, from: Address, to: Address) -> Option<(&'static str, FlowDirection)> {
        let from_cex = get_cex_by_address(from);
        let to_cex = get_cex_by_address(to);
        
        match (from_cex, to_cex) {
            (Some(from_info), Some(to_info)) => {
                if from_info.exchange == to_info.exchange {
                    // Internal transfer within same exchange
                    Some((from_info.exchange, FlowDirection::Internal))
                } else {
                    // Inter-exchange transfer (treat as outflow from sender)
                    Some((from_info.exchange, FlowDirection::Outflow))
                }
            }
            (Some(from_info), None) => {
                // Withdrawal from CEX
                Some((from_info.exchange, FlowDirection::Outflow))
            }
            (None, Some(to_info)) => {
                // Deposit to CEX
                Some((to_info.exchange, FlowDirection::Inflow))
            }
            (None, None) => None,
        }
    }
    
    /// Check if transfer involves any CEX
    pub fn involves_cex(&self, from: Address, to: Address) -> bool {
        is_cex_address(from) || is_cex_address(to)
    }
    
    /// Get net flow for an exchange between two blocks
    /// Note: This would require transaction data, which we don't have direct access to
    /// This is a placeholder for the interface
    pub async fn get_exchange_net_flow(
        &self,
        exchange_name: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<i128> {
        // This would need to process transactions in the block range
        // For now, we can only track balance changes
        
        if let Some(addresses) = get_exchange_addresses(exchange_name) {
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
            
            let net_flow = if new_balance >= old_balance {
                (new_balance - old_balance).to_string().parse::<i128>().unwrap_or(i128::MAX)
            } else {
                -(old_balance - new_balance).to_string().parse::<i128>().unwrap_or(i128::MIN)
            };
            
            Ok(net_flow)
        } else {
            Ok(0)
        }
    }
    
    /// Identify large deposits to CEX (useful for tracking)
    pub fn is_large_cex_deposit(&self, to: Address, amount: U256) -> bool {
        if is_cex_address(to) {
            // Consider large if > 100 ETH
            let threshold = U256::from(100) * U256::from(10).pow(U256::from(18));
            amount > threshold
        } else {
            false
        }
    }
    
    /// Identify large withdrawals from CEX
    pub fn is_large_cex_withdrawal(&self, from: Address, amount: U256) -> bool {
        if is_cex_address(from) {
            // Consider large if > 100 ETH
            let threshold = U256::from(100) * U256::from(10).pow(U256::from(18));
            amount > threshold
        } else {
            false
        }
    }
    
    /// Calculate flows for all CEX exchanges between two blocks
    pub async fn calculate_flows_between_blocks(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<CexFlowSummary> {
        let mut exchange_flows = HashMap::new();
        let mut total_inflow = 0.0;
        let mut total_outflow = 0.0;
        
        // Calculate flow for each exchange
        for (exchange_name, _) in exchange_stats() {
            if let Some(addresses) = get_exchange_addresses(exchange_name) {
                let mut exchange_old_balance = U256::ZERO;
                let mut exchange_new_balance = U256::ZERO;
                let mut unique_addrs = std::collections::HashSet::new();
                
                // Sum balances across all addresses for this exchange
                for address in addresses {
                    // Get old balance
                    let old = self.chain_query.account
                        .get_balance(*address, Some(from_block))
                        .await
                        .unwrap_or(U256::ZERO);
                    exchange_old_balance = exchange_old_balance.saturating_add(old);
                    
                    // Get new balance
                    let new = self.chain_query.account
                        .get_balance(*address, Some(to_block))
                        .await
                        .unwrap_or(U256::ZERO);
                    exchange_new_balance = exchange_new_balance.saturating_add(new);
                    
                    // Track unique addresses
                    unique_addrs.insert(*address);
                }
                
                // Calculate flow
                let old_eth = format_token_amount(exchange_old_balance, 18);
                let new_eth = format_token_amount(exchange_new_balance, 18);
                
                let (inflow, outflow) = if new_eth > old_eth {
                    // Net inflow (deposits)
                    (new_eth - old_eth, 0.0)
                } else if old_eth > new_eth {
                    // Net outflow (withdrawals)
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
                    
                    exchange_flows.insert(
                        exchange_name.to_string(),
                        CexFlowData {
                            exchange: exchange_name.to_string(),
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
        
        Ok(CexFlowSummary {
            from_block,
            to_block,
            exchange_flows,
            total_inflow_eth: total_inflow,
            total_outflow_eth: total_outflow,
            total_net_flow_eth: total_inflow - total_outflow,
        })
    }
}