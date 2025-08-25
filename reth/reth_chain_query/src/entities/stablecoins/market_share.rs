/// Stablecoin market share analysis
/// 
/// Calculates market share, concentration metrics, and whale analysis
/// for all stablecoins.

use crate::{ChainQuery, Result};
use crate::entities::common::format_token_amount;
use super::addresses::{STABLECOINS, StablecoinInfo};
use alloy_primitives::{Address, U256};
use std::sync::Arc;
use std::collections::HashMap;

/// Market analysis result for a single stablecoin
#[derive(Debug, Clone)]
pub struct StablecoinMarketData {
    pub info: &'static StablecoinInfo,
    pub total_supply: U256,
    pub total_supply_formatted: f64,
    pub market_share_percent: f64,
    pub rank: usize,
}

/// Overall market analysis
#[derive(Debug, Clone)]
pub struct MarketAnalysis {
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
    pub unit_name: String,  // "US Dollar", "Euro", etc.
    pub tokens: Vec<StablecoinMarketData>,
    pub total_supply_in_unit: f64,  // Total in that unit's denomination
}

/// Stablecoin market analyzer
pub struct StablecoinMarketAnalyzer {
    chain_query: Arc<ChainQuery>,
}

impl StablecoinMarketAnalyzer {
    /// Create new market analyzer
    pub fn new(chain_query: Arc<ChainQuery>) -> Self {
        Self { chain_query }
    }
    
    /// Analyze stablecoin market share
    pub async fn analyze_market(&self, block_number: Option<u64>) -> Result<MarketAnalysis> {
        let block = block_number.unwrap_or(self.chain_query.get_latest_block()?);
        
        // Collect supply data for all stablecoins
        let mut market_data = Vec::new();
        let mut total_market_supply = 0.0;
        
        for stablecoin_info in STABLECOINS {
            // Get total supply
            let total_supply = self.chain_query.token
                .get_erc20_total_supply(stablecoin_info.address, Some(block))
                .await
                .unwrap_or(U256::ZERO);
            
            let total_supply_formatted = format_token_amount(total_supply, stablecoin_info.decimals);
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
                data.market_share_percent = (data.total_supply_formatted / total_market_supply) * 100.0;
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
        // Sum of squared market shares - higher means more concentrated
        let herfindahl_index: f64 = market_data
            .iter()
            .map(|d| d.market_share_percent * d.market_share_percent)
            .sum();
        
        Ok(MarketAnalysis {
            block_number: block,
            stablecoins: market_data,
            total_market_supply,
            top_3_concentration,
            top_5_concentration,
            herfindahl_index,
        })
    }
    
    /// Get top N stablecoins by market share
    pub async fn get_top_stablecoins(&self, n: usize, block_number: Option<u64>) -> Result<Vec<StablecoinMarketData>> {
        let analysis = self.analyze_market(block_number).await?;
        Ok(analysis.stablecoins.into_iter().take(n).collect())
    }
    
    /// Check if market is concentrated (top 3 > 80%)
    pub async fn is_market_concentrated(&self, block_number: Option<u64>) -> Result<bool> {
        let analysis = self.analyze_market(block_number).await?;
        Ok(analysis.top_3_concentration > 80.0)
    }
    
    /// Analyze stablecoin market grouped by currency unit
    /// Each unit shows totals in its own denomination (USD, EUR, JPY, etc.)
    pub async fn analyze_market_by_unit(&self, block_number: Option<u64>) -> Result<MarketByUnitAnalysis> {
        let block = block_number.unwrap_or(self.chain_query.get_latest_block()?);
        
        // Group stablecoins by unit
        let mut units_map: HashMap<String, Vec<StablecoinMarketData>> = HashMap::new();
        
        for stablecoin_info in STABLECOINS {
            // Get total supply
            let total_supply = self.chain_query.token
                .get_erc20_total_supply(stablecoin_info.address, Some(block))
                .await
                .unwrap_or(U256::ZERO);
            
            let total_supply_formatted = format_token_amount(total_supply, stablecoin_info.decimals);
            
            // Skip if no supply
            if total_supply_formatted == 0.0 {
                continue;
            }
            
            let market_data = StablecoinMarketData {
                info: stablecoin_info,
                total_supply,
                total_supply_formatted,
                market_share_percent: 0.0, // Will calculate within unit
                rank: 0,
            };
            
            // Group by unit
            let unit = stablecoin_info.unit;
            units_map.entry(unit.to_string())
                .or_insert_with(Vec::new)
                .push(market_data);
        }
        
        // Process each unit group
        let mut units = HashMap::new();
        
        for (unit_name, mut tokens) in units_map {
            // Calculate total supply for this unit
            let total_supply_in_unit: f64 = tokens.iter()
                .map(|t| t.total_supply_formatted)
                .sum();
            
            // Calculate market share within this unit
            for token in &mut tokens {
                if total_supply_in_unit > 0.0 {
                    token.market_share_percent = (token.total_supply_formatted / total_supply_in_unit) * 100.0;
                }
            }
            
            // Sort by supply within unit (descending)
            tokens.sort_by(|a, b| {
                b.total_supply_formatted
                    .partial_cmp(&a.total_supply_formatted)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            
            // Assign ranks within unit
            for (i, token) in tokens.iter_mut().enumerate() {
                token.rank = i + 1;
            }
            
            units.insert(unit_name.clone(), UnitMarketData {
                unit_name,
                tokens,
                total_supply_in_unit,
            });
        }
        
        Ok(MarketByUnitAnalysis {
            block_number: block,
            units,
        })
    }
}