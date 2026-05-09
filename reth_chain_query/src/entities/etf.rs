/// Exchange-Traded Fund (ETF) holdings tracking and analysis
///
/// Provides comprehensive ETF analytics including:
/// - ETH holdings tracking across providers
/// - Market share analysis
/// - Provider rankings and concentration
use alloy_primitives::{Address, U256};
use eyre::Result;
use std::collections::HashMap;

use super::types::format_token_amount;
use crate::common_addresses::etf::{get_provider_addresses, provider_stats};
use crate::provider::RethQueryProvider;

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

impl RethQueryProvider {
    /// Get ETH holdings for all ETF providers
    pub async fn get_all_etf_holdings(&self, block: Option<u64>) -> Result<EtfHoldingsSummary> {
        let block_number = block.unwrap_or(self.get_latest_block()?);
        let mut providers = Vec::new();
        let mut total_etf_eth = U256::ZERO;

        // Get holdings for all providers
        for (provider_name, _count) in provider_stats() {
            if let Some(addresses) = get_provider_addresses(provider_name) {
                // Batch get ETH balances
                let balances = self
                    .get_eth_balances_for_multiple_addresses(addresses.to_vec(), Some(block_number))
                    .await?;

                let total_eth_holdings: U256 = balances.iter().sum();
                total_etf_eth = total_etf_eth.saturating_add(total_eth_holdings);
                let total_eth_formatted = format_token_amount(total_eth_holdings, 18);

                providers.push(ProviderHoldings {
                    name: provider_name,
                    address_count: addresses.len(),
                    total_eth_holdings,
                    total_eth_formatted,
                    market_share_percent: 0.0,
                });
            }
        }

        // Calculate market share percentages
        let total_etf_eth_formatted = format_token_amount(total_etf_eth, 18);
        for provider in &mut providers {
            if total_etf_eth_formatted > 0.0 {
                provider.market_share_percent =
                    (provider.total_eth_formatted / total_etf_eth_formatted) * 100.0;
            }
        }

        // Sort providers by ETH holdings (descending)
        providers.sort_by(|a, b| b.total_eth_holdings.cmp(&a.total_eth_holdings));

        let provider_rankings: Vec<(&'static str, f64)> = providers
            .iter()
            .map(|p| (p.name, p.total_eth_formatted))
            .collect();

        Ok(EtfHoldingsSummary {
            block_number,
            providers,
            total_etf_eth,
            total_etf_eth_formatted,
            provider_rankings,
        })
    }

    /// Get holdings for a specific ETF provider
    pub async fn get_provider_holdings(
        &self,
        provider: &str,
        tokens: Vec<Address>,
        block: Option<u64>,
    ) -> Result<super::cex::ExchangeBalance> {
        let block_number = block.unwrap_or(self.get_latest_block()?);

        let addresses = get_provider_addresses(provider)
            .ok_or_else(|| eyre::eyre!("Unknown ETF provider: {}", provider))?;

        // Get ETH balances
        let eth_balances = self
            .get_eth_balances_for_multiple_addresses(addresses.to_vec(), Some(block_number))
            .await?;

        let total_eth_balance: U256 = eth_balances.iter().sum();
        let total_eth_formatted = format_token_amount(total_eth_balance, 18);

        // Get token balances if requested
        let mut token_balances = HashMap::new();
        if !tokens.is_empty() {
            for token in tokens {
                let balances = self
                    .batch_get_balances_for_token_holder_pairs(
                        addresses.iter().map(|&addr| (token, addr)).collect(),
                        Some(block_number),
                    )
                    .await?;

                let total_token_balance: U256 = balances.iter().sum();

                if total_token_balance > U256::ZERO {
                    token_balances.insert(token, total_token_balance);
                }
            }
        }

        // Reuse ExchangeBalance structure since it has the same fields
        Ok(super::cex::ExchangeBalance {
            name: provider.to_string(),
            address_count: addresses.len(),
            total_eth_balance,
            total_eth_formatted,
            token_balances,
        })
    }

    /// Get balance changes for an ETF provider between two blocks
    pub async fn get_etf_balance_changes(
        &self,
        provider: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<(U256, U256, i128)> {
        let addresses = get_provider_addresses(provider)
            .ok_or_else(|| eyre::eyre!("Unknown ETF provider: {}", provider))?;

        // Get ETH balances at both blocks
        let before_balances = self
            .get_eth_balances_for_multiple_addresses(addresses.to_vec(), Some(from_block))
            .await?;
        let total_before: U256 = before_balances.iter().sum();

        let after_balances = self
            .get_eth_balances_for_multiple_addresses(addresses.to_vec(), Some(to_block))
            .await?;
        let total_after: U256 = after_balances.iter().sum();

        let change = if total_after >= total_before {
            (total_after - total_before)
                .to_string()
                .parse::<i128>()
                .unwrap_or(i128::MAX)
        } else {
            -(total_before - total_after)
                .to_string()
                .parse::<i128>()
                .unwrap_or(i128::MIN)
        };

        Ok((total_before, total_after, change))
    }
}
