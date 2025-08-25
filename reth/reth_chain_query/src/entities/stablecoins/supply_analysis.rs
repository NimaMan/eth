/// Stablecoin supply tracking and analysis
/// 
/// Tracks total supply changes, mints, burns, and supply velocity.

use crate::{ChainQuery, Result};
use crate::entities::common::format_token_amount;
use super::addresses::{get_stablecoin_by_address, StablecoinInfo};
use alloy_primitives::{Address, U256};
use std::sync::Arc;
use std::collections::HashMap;

/// Supply change event
#[derive(Debug, Clone)]
pub struct SupplyChange {
    pub stablecoin: &'static StablecoinInfo,
    pub block_number: u64,
    pub old_supply: U256,
    pub new_supply: U256,
    pub change_amount: i128,
    pub percent_change: f64,
    pub is_mint: bool,
    pub is_burn: bool,
}

/// Supply tracking result
#[derive(Debug, Clone)]
pub struct SupplySnapshot {
    pub block_number: u64,
    pub supplies: HashMap<Address, U256>,
    pub total_market_supply: U256,
}

/// Supply change data for a stablecoin between blocks
#[derive(Debug, Clone)]
pub struct StablecoinSupplyChangeData {
    pub token_symbol: String,
    pub token_address: Address,
    pub old_supply: f64,
    pub new_supply: f64,
    pub minted: f64,
    pub burned: f64,
    pub net_change: f64,
    pub percent_change: f64,
}

/// Aggregated supply changes for all stablecoins
#[derive(Debug, Clone)]
pub struct StablecoinSupplyChangeSummary {
    pub from_block: u64,
    pub to_block: u64,
    pub token_changes: HashMap<String, StablecoinSupplyChangeData>,
    pub total_minted: f64,
    pub total_burned: f64,
    pub total_net_change: f64,
}

/// Stablecoin supply tracker
pub struct StablecoinSupplyTracker {
    chain_query: Arc<ChainQuery>,
}

impl StablecoinSupplyTracker {
    /// Create new supply tracker
    pub fn new(chain_query: Arc<ChainQuery>) -> Self {
        Self { chain_query }
    }
    
    /// Get current supply snapshot for all stablecoins
    pub async fn get_supply_snapshot(&self, block_number: Option<u64>) -> Result<SupplySnapshot> {
        let block = block_number.unwrap_or(self.chain_query.get_latest_block()?);
        let mut supplies = HashMap::new();
        let mut total_market_supply = U256::ZERO;
        
        for stablecoin in super::addresses::STABLECOINS {
            let supply = self.chain_query.token
                .get_erc20_total_supply(stablecoin.address, Some(block))
                .await
                .unwrap_or(U256::ZERO);
            
            supplies.insert(stablecoin.address, supply);
            total_market_supply = total_market_supply.saturating_add(supply);
        }
        
        Ok(SupplySnapshot {
            block_number: block,
            supplies,
            total_market_supply,
        })
    }
    
    /// Calculate supply changes between two blocks
    pub async fn calculate_supply_changes(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<SupplyChange>> {
        let old_snapshot = self.get_supply_snapshot(Some(from_block)).await?;
        let new_snapshot = self.get_supply_snapshot(Some(to_block)).await?;
        
        let mut changes = Vec::new();
        
        for stablecoin in super::addresses::STABLECOINS {
            let old_supply = old_snapshot.supplies.get(&stablecoin.address)
                .copied()
                .unwrap_or(U256::ZERO);
            let new_supply = new_snapshot.supplies.get(&stablecoin.address)
                .copied()
                .unwrap_or(U256::ZERO);
            
            if old_supply != new_supply {
                let (change_amount, is_mint, is_burn) = if new_supply > old_supply {
                    let diff = new_supply - old_supply;
                    (diff.to_string().parse::<i128>().unwrap_or(i128::MAX), true, false)
                } else {
                    let diff = old_supply - new_supply;
                    (-diff.to_string().parse::<i128>().unwrap_or(i128::MIN), false, true)
                };
                
                let old_formatted = format_token_amount(old_supply, stablecoin.decimals);
                let new_formatted = format_token_amount(new_supply, stablecoin.decimals);
                let percent_change = if old_formatted > 0.0 {
                    ((new_formatted - old_formatted) / old_formatted) * 100.0
                } else {
                    100.0
                };
                
                changes.push(SupplyChange {
                    stablecoin,
                    block_number: to_block,
                    old_supply,
                    new_supply,
                    change_amount,
                    percent_change,
                    is_mint,
                    is_burn,
                });
            }
        }
        
        // Sort by absolute change amount (descending)
        changes.sort_by_key(|c| -c.change_amount.abs());
        
        Ok(changes)
    }
    
    /// Get supply for a specific stablecoin
    pub async fn get_stablecoin_supply(
        &self,
        address: Address,
        block_number: Option<u64>,
    ) -> Result<Option<(U256, f64)>> {
        if let Some(info) = get_stablecoin_by_address(address) {
            let block = block_number.unwrap_or(self.chain_query.get_latest_block()?);
            let supply = self.chain_query.token
                .get_erc20_total_supply(address, Some(block))
                .await?;
            let formatted = format_token_amount(supply, info.decimals);
            Ok(Some((supply, formatted)))
        } else {
            Ok(None)
        }
    }
    
    /// Track supply velocity (rate of change) over a range
    pub async fn calculate_supply_velocity(
        &self,
        address: Address,
        start_block: u64,
        end_block: u64,
        sample_interval: u64, // Blocks between samples
    ) -> Result<Vec<f64>> {
        if get_stablecoin_by_address(address).is_none() {
            return Ok(Vec::new());
        }
        
        let mut velocities = Vec::new();
        let mut current_block = start_block;
        let mut prev_supply = None;
        
        while current_block <= end_block {
            if let Ok(supply) = self.chain_query.token
                .get_erc20_total_supply(address, Some(current_block))
                .await 
            {
                if let Some(prev) = prev_supply {
                    // Calculate rate of change per block
                    let blocks_elapsed = sample_interval as f64;
                    let supply_change = if supply > prev {
                        let diff: U256 = supply - prev;
                        diff.to_string().parse::<f64>().unwrap_or(0.0)
                    } else {
                        let diff: U256 = prev - supply;
                        -diff.to_string().parse::<f64>().unwrap_or(0.0)
                    };
                    let velocity = supply_change / blocks_elapsed;
                    velocities.push(velocity);
                }
                prev_supply = Some(supply);
            }
            
            current_block += sample_interval;
        }
        
        Ok(velocities)
    }
    
    /// Calculate supply changes for all stablecoins between two blocks
    pub async fn calculate_supply_changes_between_blocks(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<StablecoinSupplyChangeSummary> {
        let mut token_changes = HashMap::new();
        let mut total_minted = 0.0;
        let mut total_burned = 0.0;
        
        // Calculate supply change for each stablecoin
        for stablecoin_info in super::addresses::STABLECOINS {
            // Get old supply
            let old_supply = self.chain_query.token
                .get_erc20_total_supply(stablecoin_info.address, Some(from_block))
                .await
                .unwrap_or(U256::ZERO);
            
            // Get new supply  
            let new_supply = self.chain_query.token
                .get_erc20_total_supply(stablecoin_info.address, Some(to_block))
                .await
                .unwrap_or(U256::ZERO);
            
            // Convert to formatted amounts
            let old_supply_formatted = format_token_amount(old_supply, stablecoin_info.decimals);
            let new_supply_formatted = format_token_amount(new_supply, stablecoin_info.decimals);
            
            // Calculate changes
            let (minted, burned) = if new_supply_formatted > old_supply_formatted {
                // Supply increased (minting)
                (new_supply_formatted - old_supply_formatted, 0.0)
            } else if old_supply_formatted > new_supply_formatted {
                // Supply decreased (burning)
                (0.0, old_supply_formatted - new_supply_formatted)
            } else {
                // No change
                (0.0, 0.0)
            };
            
            let net_change = minted - burned;
            let percent_change = if old_supply_formatted > 0.0 {
                (net_change / old_supply_formatted) * 100.0
            } else if new_supply_formatted > 0.0 {
                100.0 // From 0 to something is 100% increase
            } else {
                0.0
            };
            
            // Only include if there was activity
            if minted > 0.0 || burned > 0.0 {
                total_minted += minted;
                total_burned += burned;
                
                token_changes.insert(
                    stablecoin_info.symbol.to_string(),
                    StablecoinSupplyChangeData {
                        token_symbol: stablecoin_info.symbol.to_string(),
                        token_address: stablecoin_info.address,
                        old_supply: old_supply_formatted,
                        new_supply: new_supply_formatted,
                        minted,
                        burned,
                        net_change,
                        percent_change,
                    }
                );
            }
        }
        
        Ok(StablecoinSupplyChangeSummary {
            from_block,
            to_block,
            token_changes,
            total_minted,
            total_burned,
            total_net_change: total_minted - total_burned,
        })
    }
}