/// ETF holdings tracking
/// 
/// Tracks ETH holdings for all ETF providers.

use crate::{ChainQuery, Result};
use crate::entities::common::format_token_amount;
use super::addresses::{get_provider_addresses, provider_stats};
use alloy_primitives::{Address, U256};
use std::sync::Arc;
use std::collections::HashMap;

/// Holdings data for a single ETF provider
#[derive(Debug, Clone)]
pub struct ProviderHoldings {
    pub name: &'static str,
    pub address_count: usize,
    pub total_eth_holdings: U256,
    pub total_eth_formatted: f64,
    pub market_share_percent: f64,
}

/// Overall ETF holdings summary
#[derive(Debug, Clone)]
pub struct EtfHoldingsSummary {
    pub block_number: u64,
    pub providers: Vec<ProviderHoldings>,
    pub total_etf_eth: U256,
    pub total_etf_eth_formatted: f64,
    pub provider_rankings: Vec<(&'static str, f64)>, // Sorted by ETH holdings
}

/// ETF holdings tracker
pub struct EtfHoldingsTracker {
    chain_query: Arc<ChainQuery>,
}

impl EtfHoldingsTracker {
    /// Create new holdings tracker
    pub fn new(chain_query: Arc<ChainQuery>) -> Self {
        Self { chain_query }
    }
    
    /// Get ETH holdings for all ETF providers
    pub async fn get_all_etf_holdings(&self, block_number: Option<u64>) -> Result<EtfHoldingsSummary> {
        let block = block_number.unwrap_or(self.chain_query.get_latest_block()?);
        let mut providers = Vec::new();
        let mut total_etf_eth = U256::ZERO;
        
        // Get holdings for all providers
        for (provider_name, _count) in provider_stats() {
            if let Some(addresses) = get_provider_addresses(provider_name) {
                let mut total_eth_holdings = U256::ZERO;
                
                // Sum ETH holdings across all addresses for this provider
                for address in addresses {
                    let balance = self.chain_query.account
                        .get_balance(*address, Some(block))
                        .await
                        .unwrap_or(U256::ZERO);
                    total_eth_holdings = total_eth_holdings.saturating_add(balance);
                }
                
                total_etf_eth = total_etf_eth.saturating_add(total_eth_holdings);
                let total_eth_formatted = format_token_amount(total_eth_holdings, 18);
                
                providers.push(ProviderHoldings {
                    name: provider_name,
                    address_count: addresses.len(),
                    total_eth_holdings,
                    total_eth_formatted,
                    market_share_percent: 0.0, // Will calculate after
                });
            }
        }
        
        // Calculate market share percentages
        let total_etf_eth_formatted = format_token_amount(total_etf_eth, 18);
        for provider in &mut providers {
            if total_etf_eth_formatted > 0.0 {
                provider.market_share_percent = (provider.total_eth_formatted / total_etf_eth_formatted) * 100.0;
            }
        }
        
        // Sort providers by ETH holdings (descending)
        providers.sort_by(|a, b| {
            b.total_eth_holdings.cmp(&a.total_eth_holdings)
        });
        
        let provider_rankings: Vec<(&'static str, f64)> = providers
            .iter()
            .map(|p| (p.name, p.total_eth_formatted))
            .collect();
        
        Ok(EtfHoldingsSummary {
            block_number: block,
            providers,
            total_etf_eth,
            total_etf_eth_formatted,
            provider_rankings,
        })
    }
    
    /// Get holdings for a specific provider
    pub async fn get_provider_holdings(
        &self,
        provider_name: &str,
        block_number: Option<u64>,
    ) -> Result<Option<ProviderHoldings>> {
        let block = block_number.unwrap_or(self.chain_query.get_latest_block()?);
        
        // Find the matching provider from our static list
        for (static_provider_name, _count) in provider_stats() {
            if static_provider_name == provider_name {
                if let Some(addresses) = get_provider_addresses(static_provider_name) {
                    let mut total_eth_holdings = U256::ZERO;
                    
                    for address in addresses {
                        let balance = self.chain_query.account
                            .get_balance(*address, Some(block))
                            .await
                            .unwrap_or(U256::ZERO);
                        total_eth_holdings = total_eth_holdings.saturating_add(balance);
                    }
                    
                    let total_eth_formatted = format_token_amount(total_eth_holdings, 18);
                    
                    return Ok(Some(ProviderHoldings {
                        name: static_provider_name, // Use the static string
                        address_count: addresses.len(),
                        total_eth_holdings,
                        total_eth_formatted,
                        market_share_percent: 0.0, // Would need total to calculate
                    }));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Get top N providers by ETH holdings
    pub async fn get_top_providers(
        &self,
        n: usize,
        block_number: Option<u64>,
    ) -> Result<Vec<(&'static str, f64)>> {
        let summary = self.get_all_etf_holdings(block_number).await?;
        Ok(summary.provider_rankings.into_iter().take(n).collect())
    }
    
    /// Compare holdings between two providers
    pub async fn compare_providers(
        &self,
        provider1: &str,
        provider2: &str,
        block_number: Option<u64>,
    ) -> Result<Option<(f64, f64)>> {
        let holdings1 = self.get_provider_holdings(provider1, block_number).await?;
        let holdings2 = self.get_provider_holdings(provider2, block_number).await?;
        
        match (holdings1, holdings2) {
            (Some(h1), Some(h2)) => Ok(Some((h1.total_eth_formatted, h2.total_eth_formatted))),
            _ => Ok(None),
        }
    }
}