/// Tax Calculator Utility Functions
/// 
/// Pure functions to calculate buy and sell taxes from state changes.
/// These can be used by any component that has access to transaction state changes.
///
/// Tax Calculation Formula:
/// - Buy Tax: (1 - tokens_received_by_buyer / tokens_sent_by_pool) × 100
/// - Sell Tax: (1 - eth_received_by_seller / eth_sent_by_pool) × 100

use alloy_primitives::{Address, U256, I256};
use reth_tx_simulator::AddressStateChange;
use std::collections::HashMap;
use crate::common::address::alloy_address_to_checksum;
use tracing::{error, debug, warn};

/// Result of tax calculation
#[derive(Debug, Clone)]
pub enum TaxCalculationResult {
    /// Successfully calculated tax (0-100%)
    Calculated(f64),
    /// Failed to determine tax due to invalid simulation results
    InvalidSimulation { reason: String },
}

/// Calculate buy tax from state changes
/// 
/// Buy Tax Formula: (1 - tokens_received_by_buyer / tokens_sent_by_pool) × 100
/// 
/// We track:
/// 1. How many tokens leave the pool (negative token_net for pool)
/// 2. How many tokens the buyer receives (positive token_net for buyer)
/// 3. The difference is the tax
/// 
/// Example: Pool sends 100 tokens, buyer receives 95 tokens = 5% buy tax
///
/// # Arguments
/// * `state_changes` - The state changes from the buy transaction
/// * `pool_address` - The address of the liquidity pool
/// * `buyer_address` - The address of the buyer
/// * `token_address` - The address of the token being bought
///
/// # Returns
/// * `TaxCalculationResult` - Either Calculated(0-100) or InvalidSimulation with reason
pub fn calculate_buy_tax(
    state_changes: &HashMap<Address, AddressStateChange>,
    pool_address: &Address,
    buyer_address: &Address,
    token_address: &Address,
) -> TaxCalculationResult {
    // Check if pool is in state changes
    let pool_changes = match state_changes.get(pool_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Pool address {} not found in state changes", pool_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Check if buyer is in state changes
    let buyer_changes = match state_changes.get(buyer_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Buyer address {} not found in state changes", buyer_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Get the checksummed token address string for lookup
    let token_addr_str = alloy_address_to_checksum(*token_address);
    
    // Get tokens sent by pool (negative value in token_net)
    let pool_token_change = match pool_changes.token_net.get(&token_addr_str) {
        Some(change) => change,
        None => {
            let reason = format!("Token {} not found in pool's token_net. Available tokens: {:?}", 
                   token_addr_str, pool_changes.token_net.keys().collect::<Vec<_>>());
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Pool loses tokens, so the value should be negative
    // We need the absolute value
    let tokens_from_pool = if *pool_token_change >= I256::ZERO {
        let reason = format!("Pool token change is positive or zero ({}) - pool should lose tokens in a buy", 
               pool_token_change);
        warn!("INVALID_SIMULATION_RESULTS: {}", reason);
        return TaxCalculationResult::InvalidSimulation { reason };
    } else {
        // Convert negative to positive by subtracting from zero
        pool_token_change.wrapping_neg()
    };
    
    // Get tokens received by buyer (positive value in token_net)
    let tokens_to_buyer = match buyer_changes.token_net.get(&token_addr_str) {
        Some(change) => change,
        None => {
            let reason = format!("Token {} not found in buyer's token_net. Available tokens: {:?}", 
                   token_addr_str, buyer_changes.token_net.keys().collect::<Vec<_>>());
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Check if buyer received any tokens
    if tokens_to_buyer.is_zero() {
        // This is 100% tax (honeypot) - buyer got nothing
        debug!("Buy tax calc: Buyer received 0 tokens while pool sent {} - this is 100% tax", tokens_from_pool);
        return TaxCalculationResult::Calculated(100.0);
    }
    
    // Calculate tax percentage
    // Convert to f64 for percentage calculation
    // tokens_from_pool is already U256, tokens_to_buyer is I256
    let from_pool_f64 = tokens_from_pool.to_string().parse::<f64>().unwrap_or(0.0);
    let to_buyer_f64 = tokens_to_buyer.unsigned_abs().to_string().parse::<f64>().unwrap_or(0.0);
    
    if from_pool_f64 > 0.0 {
        let tax_percent = (1.0 - (to_buyer_f64 / from_pool_f64)) * 100.0;
        TaxCalculationResult::Calculated(tax_percent.max(0.0))
    } else {
        let reason = "Pool sent zero tokens - invalid simulation state".to_string();
        warn!("INVALID_SIMULATION_RESULTS: {}", reason);
        TaxCalculationResult::InvalidSimulation { reason }
    }
}

/// Calculate sell tax from state changes
/// 
/// Sell Tax Formula: (1 - eth_received_by_seller / eth_sent_by_pool) × 100
/// 
/// We track:
/// 1. How much ETH leaves the pool (negative eth_net for pool)
/// 2. How much ETH the seller receives (positive eth_net for seller)
/// 3. The difference is the tax
/// 
/// Example: Pool sends 1 ETH, seller receives 0.9 ETH = 10% sell tax
///
/// # Arguments
/// * `state_changes` - The state changes from the sell transaction
/// * `pool_address` - The address of the liquidity pool
/// * `seller_address` - The address of the seller
///
/// # Returns
/// * `TaxCalculationResult` - Either Calculated(0-100) or InvalidSimulation with reason
pub fn calculate_sell_tax(
    state_changes: &HashMap<Address, AddressStateChange>,
    pool_address: &Address,
    seller_address: &Address,
) -> TaxCalculationResult {
    // Check if pool is in state changes
    let pool_changes = match state_changes.get(pool_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Pool address {} not found in state changes", pool_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Check if seller is in state changes
    let seller_changes = match state_changes.get(seller_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Seller address {} not found in state changes", seller_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Get ETH sent by pool (negative value in eth_net)
    let pool_eth_change = pool_changes.eth_net;
    
    // Pool loses ETH, so the value should be negative
    // We need the absolute value
    let eth_from_pool = if pool_eth_change >= I256::ZERO {
        let reason = format!("Pool ETH change is positive or zero ({}) - pool should lose ETH in a sell", 
               pool_eth_change);
        warn!("INVALID_SIMULATION_RESULTS: {}", reason);
        return TaxCalculationResult::InvalidSimulation { reason };
    } else {
        // Convert negative to positive by subtracting from zero
        pool_eth_change.wrapping_neg()
    };
    
    // Get ETH received by seller (positive value in eth_net)
    let eth_to_seller = seller_changes.eth_net;
    
    // Check if seller received any ETH
    if eth_to_seller.is_zero() {
        // This is 100% tax (honeypot) - seller got nothing
        debug!("Sell tax calc: Seller received 0 ETH while pool sent {} - this is 100% tax", eth_from_pool);
        return TaxCalculationResult::Calculated(100.0);
    }
    
    // Calculate tax percentage
    // Convert to f64 for percentage calculation
    // eth_from_pool is already U256, eth_to_seller is I256
    let from_pool_f64 = eth_from_pool.to_string().parse::<f64>().unwrap_or(0.0);
    let to_seller_f64 = eth_to_seller.unsigned_abs().to_string().parse::<f64>().unwrap_or(0.0);
    
    if from_pool_f64 > 0.0 {
        let tax_percent = (1.0 - (to_seller_f64 / from_pool_f64)) * 100.0;
        TaxCalculationResult::Calculated(tax_percent.max(0.0))
    } else {
        let reason = "Pool sent zero ETH - invalid simulation state".to_string();
        warn!("INVALID_SIMULATION_RESULTS: {}", reason);
        TaxCalculationResult::InvalidSimulation { reason }
    }
}

/// Calculate buy tax using movements data (alternative method)
/// 
/// This method uses the detailed movements tracking from AddressStateChange
/// which provides more granular information about token flows.
pub fn calculate_buy_tax_from_movements(
    state_changes: &HashMap<Address, AddressStateChange>,
    pool_address: &Address,
    buyer_address: &Address,
) -> TaxCalculationResult {
    let pool_changes = match state_changes.get(pool_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Pool address {} not found in state changes", pool_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    let buyer_changes = match state_changes.get(buyer_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Buyer address {} not found in state changes", buyer_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Find the token address by checking buyer's incoming tokens
    for (token_addr, buyer_token_movements) in &buyer_changes.movements.tokens {
        // Calculate tokens received by buyer
        let tokens_to_buyer: U256 = buyer_token_movements.incoming
            .values()
            .map(|e| e.amount)
            .fold(U256::ZERO, |acc, x| acc + x);
        
        if tokens_to_buyer > U256::ZERO {
            // Check pool's outgoing tokens for the same token
            if let Some(pool_token_movements) = pool_changes.movements.tokens.get(token_addr) {
                let tokens_from_pool: U256 = pool_token_movements.outgoing
                    .values()
                    .map(|e| e.amount)
                    .fold(U256::ZERO, |acc, x| acc + x);
                
                if tokens_from_pool > U256::ZERO {
                    // Calculate tax percentage
                    let from_pool_f64 = tokens_from_pool.to_string().parse::<f64>().unwrap_or(0.0);
                    let to_buyer_f64 = tokens_to_buyer.to_string().parse::<f64>().unwrap_or(0.0);
                    
                    if from_pool_f64 > 0.0 {
                        let tax_percent = (1.0 - (to_buyer_f64 / from_pool_f64)) * 100.0;
                        return TaxCalculationResult::Calculated(tax_percent.max(0.0));
                    }
                }
            }
        }
    }
    
    let reason = "No token movements found in state changes".to_string();
    warn!("INVALID_SIMULATION_RESULTS: {}", reason);
    TaxCalculationResult::InvalidSimulation { reason }
}

/// Calculate sell tax using movements data (alternative method)
/// 
/// This method uses the detailed movements tracking from AddressStateChange
/// which provides more granular information about ETH flows.
pub fn calculate_sell_tax_from_movements(
    state_changes: &HashMap<Address, AddressStateChange>,
    pool_address: &Address,
    seller_address: &Address,
) -> TaxCalculationResult {
    let pool_changes = match state_changes.get(pool_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Pool address {} not found in state changes", pool_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    let seller_changes = match state_changes.get(seller_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Seller address {} not found in state changes", seller_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Calculate ETH received by seller
    let eth_to_seller: U256 = seller_changes.movements.denom.incoming
        .values()
        .map(|e| e.amount)
        .fold(U256::ZERO, |acc, x| acc + x);
    
    if eth_to_seller > U256::ZERO {
        // Calculate ETH sent from pool
        let eth_from_pool: U256 = pool_changes.movements.denom.outgoing
            .values()
            .map(|e| e.amount)
            .fold(U256::ZERO, |acc, x| acc + x);
        
        if eth_from_pool > U256::ZERO {
            // Calculate tax percentage
            let from_pool_f64 = eth_from_pool.to_string().parse::<f64>().unwrap_or(0.0);
            let to_seller_f64 = eth_to_seller.to_string().parse::<f64>().unwrap_or(0.0);
            
            if from_pool_f64 > 0.0 {
                let tax_percent = (1.0 - (to_seller_f64 / from_pool_f64)) * 100.0;
                return TaxCalculationResult::Calculated(tax_percent.max(0.0));
            }
        }
    }
    
    let reason = "No ETH movements found in state changes".to_string();
    warn!("INVALID_SIMULATION_RESULTS: {}", reason);
    TaxCalculationResult::InvalidSimulation { reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_buy_tax_calculation() {
        // TODO: Add unit tests
    }
    
    #[test]
    fn test_sell_tax_calculation() {
        // TODO: Add unit tests
    }
}