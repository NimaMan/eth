/// Tax Calculator Utility Functions
/// 
/// Pure functions to calculate buy and sell taxes from state changes.
/// These can be used by any component that has access to transaction state changes.
///
/// Tax Calculation Formula:
/// - Buy Tax: (1 - tokens_received_by_buyer / tokens_sent_by_pool) × 100
/// - Sell Tax: (1 - eth_received_by_seller / eth_sent_by_pool) × 100

use alloy_primitives::{Address, U256};
use reth_tx_simulator::AddressStateChange;
use std::collections::HashMap;

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
/// * `Option<f64>` - The calculated buy tax percentage (0-100), or None if calculation fails
pub fn calculate_buy_tax(
    state_changes: &HashMap<Address, AddressStateChange>,
    pool_address: &Address,
    buyer_address: &Address,
    token_address: &Address,
) -> Option<f64> {
    let pool_changes = state_changes.get(pool_address)?;
    let buyer_changes = state_changes.get(buyer_address)?;
    
    // Get the token address string for lookup
    let token_addr_str = format!("{:#x}", token_address);
    
    // Get tokens sent by pool (negative value in token_net)
    let pool_token_change = pool_changes.token_net.get(&token_addr_str)?;
    
    // Pool loses tokens, so the value should be negative or zero
    // We need the absolute value
    let tokens_from_pool = if *pool_token_change > U256::ZERO {
        // This shouldn't happen in a normal buy, pool should lose tokens
        return None;
    } else {
        // Negate the negative value to get positive amount
        U256::ZERO - *pool_token_change
    };
    
    // Get tokens received by buyer (positive value in token_net)
    let tokens_to_buyer = buyer_changes.token_net.get(&token_addr_str)?;
    
    // Buyer should receive tokens (positive value)
    if *tokens_to_buyer == U256::ZERO {
        return None;
    }
    
    // Calculate tax percentage
    // Convert to f64 for percentage calculation
    let from_pool_f64 = tokens_from_pool.to_string().parse::<f64>().unwrap_or(0.0);
    let to_buyer_f64 = tokens_to_buyer.to_string().parse::<f64>().unwrap_or(0.0);
    
    if from_pool_f64 > 0.0 {
        let tax_percent = (1.0 - (to_buyer_f64 / from_pool_f64)) * 100.0;
        Some(tax_percent.max(0.0))
    } else {
        None
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
/// * `Option<f64>` - The calculated sell tax percentage (0-100), or None if calculation fails
pub fn calculate_sell_tax(
    state_changes: &HashMap<Address, AddressStateChange>,
    pool_address: &Address,
    seller_address: &Address,
) -> Option<f64> {
    let pool_changes = state_changes.get(pool_address)?;
    let seller_changes = state_changes.get(seller_address)?;
    
    // Get ETH sent by pool (negative value in eth_net)
    let pool_eth_change = pool_changes.eth_net;
    
    // Pool loses ETH, so the value should be negative
    // We need the absolute value
    let eth_from_pool = if pool_eth_change > U256::ZERO {
        // This shouldn't happen in a normal sell, pool should lose ETH
        return None;
    } else {
        // Negate the negative value to get positive amount
        U256::ZERO - pool_eth_change
    };
    
    // Get ETH received by seller (positive value in eth_net)
    let eth_to_seller = seller_changes.eth_net;
    
    // Seller should receive ETH (positive value)
    if eth_to_seller == U256::ZERO {
        return None;
    }
    
    // Calculate tax percentage
    // Convert to f64 for percentage calculation
    let from_pool_f64 = eth_from_pool.to_string().parse::<f64>().unwrap_or(0.0);
    let to_seller_f64 = eth_to_seller.to_string().parse::<f64>().unwrap_or(0.0);
    
    if from_pool_f64 > 0.0 {
        let tax_percent = (1.0 - (to_seller_f64 / from_pool_f64)) * 100.0;
        Some(tax_percent.max(0.0))
    } else {
        None
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
) -> Option<f64> {
    let pool_changes = state_changes.get(pool_address)?;
    let buyer_changes = state_changes.get(buyer_address)?;
    
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
                        return Some(tax_percent.max(0.0));
                    }
                }
            }
        }
    }
    
    None
}

/// Calculate sell tax using movements data (alternative method)
/// 
/// This method uses the detailed movements tracking from AddressStateChange
/// which provides more granular information about ETH flows.
pub fn calculate_sell_tax_from_movements(
    state_changes: &HashMap<Address, AddressStateChange>,
    pool_address: &Address,
    seller_address: &Address,
) -> Option<f64> {
    let pool_changes = state_changes.get(pool_address)?;
    let seller_changes = state_changes.get(seller_address)?;
    
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
                return Some(tax_percent.max(0.0));
            }
        }
    }
    
    None
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