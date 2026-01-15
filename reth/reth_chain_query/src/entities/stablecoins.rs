/// Stablecoin market analysis and tracking
///
/// Provides comprehensive stablecoin analytics including:
/// - Market share and concentration metrics
/// - Supply tracking across different currency units
/// - Historical trends and changes
use alloy_primitives::U256;
use eyre::Result;
use std::collections::HashMap;

use super::types::format_token_amount;
use crate::common_addresses::stablecoins::{StablecoinInfo, STABLECOINS};
use crate::provider::RethQueryProvider;

/// Market data for a single stablecoin
#[derive(Debug, Clone)]
pub struct StablecoinMarketData {
    pub info: &'static StablecoinInfo,
    pub total_supply: U256,
    pub total_supply_formatted: f64,
    pub market_share_percent: f64,
    pub rank: usize,
}

/// Overall stablecoin market analysis
#[derive(Debug, Clone)]
pub struct StablecoinMarketAnalysis {
    pub block_number: u64,
    pub stablecoins: Vec<StablecoinMarketData>,
    pub total_market_supply: f64,
    pub top_3_concentration: f64,
    pub top_5_concentration: f64,
    pub herfindahl_index: f64, // Market concentration metric
}

/// Market analysis grouped by currency unit
#[derive(Debug, Clone)]
pub struct MarketByUnitAnalysis {
    pub block_number: u64,
    pub units: HashMap<String, UnitMarketData>,
}

/// Market data for a specific currency unit
#[derive(Debug, Clone)]
pub struct UnitMarketData {
    pub unit_name: String,
    pub tokens: Vec<StablecoinMarketData>,
    pub total_supply_in_unit: f64,
}

impl RethQueryProvider {
    /// Analyze stablecoin market share
    pub async fn get_stablecoin_market_share(
        &self,
        block: Option<u64>,
    ) -> Result<StablecoinMarketAnalysis> {
        let block_number = block.unwrap_or(self.get_latest_block()?);

        // Collect supply data for all stablecoins
        let mut market_data = Vec::new();
        let mut total_market_supply = 0.0;

        for stablecoin_info in STABLECOINS {
            // Get total supply using view function
            let total_supply = self
                .execute_contract_method(
                    stablecoin_info.address,
                    "totalSupply",
                    &[],
                    Some(block_number),
                )
                .await
                .unwrap_or(U256::ZERO);

            let total_supply_formatted =
                format_token_amount(total_supply, stablecoin_info.decimals);
            total_market_supply += total_supply_formatted;

            market_data.push(StablecoinMarketData {
                info: stablecoin_info,
                total_supply,
                total_supply_formatted,
                market_share_percent: 0.0, // Will calculate after
                rank: 0,
            });
        }

        // Calculate market share percentages and sort by supply
        for data in &mut market_data {
            if total_market_supply > 0.0 {
                data.market_share_percent =
                    (data.total_supply_formatted / total_market_supply) * 100.0;
            }
        }

        // Sort by market cap (descending)
        market_data.sort_by(|a, b| {
            b.total_supply_formatted
                .partial_cmp(&a.total_supply_formatted)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Assign ranks
        for (i, data) in market_data.iter_mut().enumerate() {
            data.rank = i + 1;
        }

        // Calculate concentration metrics
        let top_3_concentration: f64 = market_data
            .iter()
            .take(3)
            .map(|d| d.market_share_percent)
            .sum();

        let top_5_concentration: f64 = market_data
            .iter()
            .take(5)
            .map(|d| d.market_share_percent)
            .sum();

        // Calculate Herfindahl-Hirschman Index (HHI)
        let herfindahl_index: f64 = market_data
            .iter()
            .map(|d| d.market_share_percent * d.market_share_percent)
            .sum();

        Ok(StablecoinMarketAnalysis {
            block_number,
            stablecoins: market_data,
            total_market_supply,
            top_3_concentration,
            top_5_concentration,
            herfindahl_index,
        })
    }

    /// Get top N stablecoins by market share
    pub async fn get_top_stablecoins(
        &self,
        n: usize,
        block: Option<u64>,
    ) -> Result<Vec<StablecoinMarketData>> {
        let analysis = self.get_stablecoin_market_share(block).await?;
        Ok(analysis.stablecoins.into_iter().take(n).collect())
    }

    /// Analyze stablecoin market grouped by currency unit
    pub async fn get_stablecoins_by_unit(
        &self,
        block: Option<u64>,
    ) -> Result<MarketByUnitAnalysis> {
        let block_number = block.unwrap_or(self.get_latest_block()?);

        // Group stablecoins by unit
        let mut units_map: HashMap<String, Vec<StablecoinMarketData>> = HashMap::new();

        for stablecoin_info in STABLECOINS {
            let total_supply = self
                .execute_contract_method(
                    stablecoin_info.address,
                    "totalSupply",
                    &[],
                    Some(block_number),
                )
                .await
                .unwrap_or(U256::ZERO);

            let total_supply_formatted =
                format_token_amount(total_supply, stablecoin_info.decimals);

            // Skip if no supply
            if total_supply_formatted == 0.0 {
                continue;
            }

            let market_data = StablecoinMarketData {
                info: stablecoin_info,
                total_supply,
                total_supply_formatted,
                market_share_percent: 0.0,
                rank: 0,
            };

            units_map
                .entry(stablecoin_info.unit.to_string())
                .or_insert_with(Vec::new)
                .push(market_data);
        }

        // Process each unit group
        let mut units = HashMap::new();

        for (unit_name, mut tokens) in units_map {
            let total_supply_in_unit: f64 = tokens.iter().map(|t| t.total_supply_formatted).sum();

            // Calculate market share within unit
            for token in &mut tokens {
                if total_supply_in_unit > 0.0 {
                    token.market_share_percent =
                        (token.total_supply_formatted / total_supply_in_unit) * 100.0;
                }
            }

            // Sort and rank
            tokens.sort_by(|a, b| {
                b.total_supply_formatted
                    .partial_cmp(&a.total_supply_formatted)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            for (i, token) in tokens.iter_mut().enumerate() {
                token.rank = i + 1;
            }

            units.insert(
                unit_name.clone(),
                UnitMarketData {
                    unit_name,
                    tokens,
                    total_supply_in_unit,
                },
            );
        }

        Ok(MarketByUnitAnalysis {
            block_number,
            units,
        })
    }
}
