/// Centralized Exchange (CEX) balance tracking and analysis
/// 
/// Provides comprehensive CEX analytics including:
/// - ETH and token balance tracking across exchanges
/// - Balance changes and flow analysis
/// - Exchange rankings and concentration metrics

use alloy_primitives::{Address, U256};
use eyre::Result;
use std::collections::HashMap;

use crate::provider::RethQueryProvider;
use crate::common_addresses::cex::{get_exchange_addresses, exchange_stats};
use super::types::format_token_amount;

/// Balance data for a single exchange
#[derive(Debug, Clone)]
pub struct ExchangeBalance {
    pub name: String,
    pub address_count: usize,
    pub total_eth_balance: U256,
    pub total_eth_formatted: f64,
    pub token_balances: HashMap<Address, U256>,
}

/// Overall CEX balance summary
#[derive(Debug, Clone)]
pub struct CexBalanceSummary {
    pub block_number: u64,
    pub exchanges: Vec<ExchangeBalance>,
    pub total_cex_eth: U256,
    pub total_cex_eth_formatted: f64,
    pub exchange_rankings: Vec<(String, f64)>, // Sorted by ETH holdings
}

impl RethQueryProvider {
    /// Get ETH balances for all CEX addresses
    pub async fn get_all_cex_balances(&self, block: Option<u64>) -> Result<CexBalanceSummary> {
        let block_number = block.unwrap_or(self.get_latest_block()?);
        let mut exchanges = Vec::new();
        let mut total_cex_eth = U256::ZERO;
        
        // Get stats for all exchanges
        for (exchange_name, _count) in exchange_stats() {
            if let Some(addresses) = get_exchange_addresses(exchange_name) {
                // Batch get ETH balances for all addresses of this exchange
                let balances = self.get_eth_balances_for_multiple_addresses(
                    addresses.to_vec(),
                    Some(block_number)
                ).await?;
                
                let total_eth_balance: U256 = balances.iter().sum();
                total_cex_eth = total_cex_eth.saturating_add(total_eth_balance);
                let total_eth_formatted = format_token_amount(total_eth_balance, 18);
                
                exchanges.push(ExchangeBalance {
                    name: exchange_name.to_string(),
                    address_count: addresses.len(),
                    total_eth_balance,
                    total_eth_formatted,
                    token_balances: HashMap::new(),
                });
            }
        }
        
        // Sort exchanges by ETH balance (descending)
        exchanges.sort_by(|a, b| {
            b.total_eth_balance.cmp(&a.total_eth_balance)
        });
        
        let exchange_rankings: Vec<(String, f64)> = exchanges
            .iter()
            .map(|e| (e.name.clone(), e.total_eth_formatted))
            .collect();
        
        let total_cex_eth_formatted = format_token_amount(total_cex_eth, 18);
        
        Ok(CexBalanceSummary {
            block_number,
            exchanges,
            total_cex_eth,
            total_cex_eth_formatted,
            exchange_rankings,
        })
    }
    
    /// Get token balances for a specific exchange
    pub async fn get_exchange_balances(
        &self,
        exchange: &str,
        tokens: Vec<Address>,
        block: Option<u64>
    ) -> Result<ExchangeBalance> {
        let block_number = block.unwrap_or(self.get_latest_block()?);
        
        let addresses = get_exchange_addresses(exchange)
            .ok_or_else(|| eyre::eyre!("Unknown exchange: {}", exchange))?;
        
        // Get ETH balances
        let eth_balances = self.get_eth_balances_for_multiple_addresses(
            addresses.to_vec(),
            Some(block_number)
        ).await?;
        
        let total_eth_balance: U256 = eth_balances.iter().sum();
        let total_eth_formatted = format_token_amount(total_eth_balance, 18);
        
        // Get token balances if requested
        let mut token_balances = HashMap::new();
        if !tokens.is_empty() {
            for token in tokens {
                let mut total_token_balance = U256::ZERO;
                
                // Batch get token balances for all exchange addresses
                let balances = self.batch_get_balances_for_token_holder_pairs(
                    addresses.iter().map(|&addr| (token, addr)).collect(),
                    Some(block_number)
                ).await?;
                
                total_token_balance = balances.iter().sum();
                
                if total_token_balance > U256::ZERO {
                    token_balances.insert(token, total_token_balance);
                }
            }
        }
        
        Ok(ExchangeBalance {
            name: exchange.to_string(),
            address_count: addresses.len(),
            total_eth_balance,
            total_eth_formatted,
            token_balances,
        })
    }
    
    /// Get balance changes for an exchange between two blocks
    pub async fn get_cex_balance_changes(
        &self,
        exchange: &str,
        from_block: u64,
        to_block: u64,
        tokens: Vec<Address>
    ) -> Result<HashMap<Address, (U256, U256, i128)>> {
        let addresses = get_exchange_addresses(exchange)
            .ok_or_else(|| eyre::eyre!("Unknown exchange: {}", exchange))?;
        
        let mut changes = HashMap::new();
        
        // ETH changes
        let eth_before_balances = self.get_eth_balances_for_multiple_addresses(
            addresses.to_vec(),
            Some(from_block)
        ).await?;
        let eth_before: U256 = eth_before_balances.iter().sum();
        
        let eth_after_balances = self.get_eth_balances_for_multiple_addresses(
            addresses.to_vec(),
            Some(to_block)
        ).await?;
        let eth_after: U256 = eth_after_balances.iter().sum();
        
        let eth_change = if eth_after >= eth_before {
            (eth_after - eth_before).to_string().parse::<i128>().unwrap_or(i128::MAX)
        } else {
            -(eth_before - eth_after).to_string().parse::<i128>().unwrap_or(i128::MIN)
        };
        
        changes.insert(Address::ZERO, (eth_before, eth_after, eth_change));
        
        // Token changes
        for token in tokens {
            let before_balances = self.batch_get_balances_for_token_holder_pairs(
                addresses.iter().map(|&addr| (token, addr)).collect(),
                Some(from_block)
            ).await?;
            let total_before: U256 = before_balances.iter().sum();
            
            let after_balances = self.batch_get_balances_for_token_holder_pairs(
                addresses.iter().map(|&addr| (token, addr)).collect(),
                Some(to_block)
            ).await?;
            let total_after: U256 = after_balances.iter().sum();
            
            let change = if total_after >= total_before {
                (total_after - total_before).to_string().parse::<i128>().unwrap_or(i128::MAX)
            } else {
                -(total_before - total_after).to_string().parse::<i128>().unwrap_or(i128::MIN)
            };
            
            changes.insert(token, (total_before, total_after, change));
        }
        
        Ok(changes)
    }
}