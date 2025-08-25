/// CEX balance tracking
/// 
/// Tracks ETH and token balances for all CEX addresses.

use crate::{ChainQuery, Result};
use crate::entities::common::format_token_amount;
use super::addresses::{get_exchange_addresses, exchange_stats};
use alloy_primitives::{Address, U256};
use std::sync::Arc;
use std::collections::HashMap;

/// Balance data for a single exchange
#[derive(Debug, Clone)]
pub struct ExchangeBalance {
    pub name: &'static str,
    pub address_count: usize,
    pub total_eth_balance: U256,
    pub total_eth_formatted: f64,
    pub token_balances: HashMap<Address, U256>, // token -> balance
}

/// Overall CEX balance summary
#[derive(Debug, Clone)]
pub struct CexBalanceSummary {
    pub block_number: u64,
    pub exchanges: Vec<ExchangeBalance>,
    pub total_cex_eth: U256,
    pub total_cex_eth_formatted: f64,
    pub exchange_rankings: Vec<(&'static str, f64)>, // Sorted by ETH holdings
}

/// CEX balance tracker
pub struct CexBalanceTracker {
    chain_query: Arc<ChainQuery>,
}

impl CexBalanceTracker {
    /// Create new balance tracker
    pub fn new(chain_query: Arc<ChainQuery>) -> Self {
        Self { chain_query }
    }
    
    /// Get ETH balances for all CEX addresses
    pub async fn get_all_cex_eth_balances(&self, block_number: Option<u64>) -> Result<CexBalanceSummary> {
        let block = block_number.unwrap_or(self.chain_query.get_latest_block()?);
        let mut exchanges = Vec::new();
        let mut total_cex_eth = U256::ZERO;
        
        // Get stats for all exchanges
        for (exchange_name, _count) in exchange_stats() {
            if let Some(addresses) = get_exchange_addresses(exchange_name) {
                let mut total_eth_balance = U256::ZERO;
                
                // Sum ETH balance across all addresses for this exchange
                for address in addresses {
                    let balance = self.chain_query.account
                        .get_balance(*address, Some(block))
                        .await
                        .unwrap_or(U256::ZERO);
                    total_eth_balance = total_eth_balance.saturating_add(balance);
                }
                
                total_cex_eth = total_cex_eth.saturating_add(total_eth_balance);
                let total_eth_formatted = format_token_amount(total_eth_balance, 18);
                
                exchanges.push(ExchangeBalance {
                    name: exchange_name,
                    address_count: addresses.len(),
                    total_eth_balance,
                    total_eth_formatted,
                    token_balances: HashMap::new(), // Can be populated if needed
                });
            }
        }
        
        // Sort exchanges by ETH balance (descending)
        exchanges.sort_by(|a, b| {
            b.total_eth_balance.cmp(&a.total_eth_balance)
        });
        
        let exchange_rankings: Vec<(&'static str, f64)> = exchanges
            .iter()
            .map(|e| (e.name, e.total_eth_formatted))
            .collect();
        
        let total_cex_eth_formatted = format_token_amount(total_cex_eth, 18);
        
        Ok(CexBalanceSummary {
            block_number: block,
            exchanges,
            total_cex_eth,
            total_cex_eth_formatted,
            exchange_rankings,
        })
    }
    
    /// Get token balances for a specific exchange
    pub async fn get_exchange_token_balances(
        &self,
        exchange_name: &str,
        token_address: Address,
        block_number: Option<u64>,
    ) -> Result<U256> {
        let block = block_number.unwrap_or(self.chain_query.get_latest_block()?);
        let mut total_balance = U256::ZERO;
        
        if let Some(addresses) = get_exchange_addresses(exchange_name) {
            for address in addresses {
                let balance = self.chain_query.token
                    .get_erc20_balance(token_address, *address, Some(block))
                    .await
                    .unwrap_or(U256::ZERO);
                total_balance = total_balance.saturating_add(balance);
            }
        }
        
        Ok(total_balance)
    }
    
    /// Get top N exchanges by ETH holdings
    pub async fn get_top_exchanges_by_eth(
        &self,
        n: usize,
        block_number: Option<u64>,
    ) -> Result<Vec<(&'static str, f64)>> {
        let summary = self.get_all_cex_eth_balances(block_number).await?;
        Ok(summary.exchange_rankings.into_iter().take(n).collect())
    }
    
    /// Check if an address is a major CEX address (top exchanges)
    pub fn is_major_cex_address(&self, address: Address) -> bool {
        if let Some(cex_info) = super::addresses::get_cex_by_address(address) {
            // Consider major if it's from top exchanges
            matches!(cex_info.exchange, "Binance" | "Coinbase" | "Kraken" | "Bitfinex" | "Huobi")
        } else {
            false
        }
    }
}