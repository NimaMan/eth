/// Tax Calculator for Trading Simulator
/// 
/// Ported from mempool_processor to calculate buy/sell taxes from address balance changes

use alloy_primitives::{Address, U256, I256};
use reth_tx_simulator::AddressBalanceChange;
use std::collections::HashMap;
use tx_processor::utils::to_checksum_address;
use tracing::{warn, debug};

/// Result of tax calculation
#[derive(Debug, Clone)]
pub enum TaxCalculationResult {
    /// Successfully calculated tax (0-100%)
    Calculated(f64),
    /// Failed to determine tax due to invalid simulation results
    InvalidSimulation { reason: String },
}

/// Calculate buy tax from address balance changes
/// 
/// Buy Tax Formula: (1 - tokens_received_by_buyer / tokens_sent_by_pool) × 100
/// 
/// We track:
/// 1. How many tokens leave the pool (negative token_net for pool)
/// 2. How many tokens the buyer receives (positive token_net for buyer)
/// 3. The difference is the tax
/// 
/// Example: Pool sends 100 tokens, buyer receives 95 tokens = 5% buy tax
pub fn calculate_buy_tax(
    address_balance_changes: &HashMap<Address, AddressBalanceChange>,
    pool_address: &Address,
    buyer_address: &Address,
    token_address: &Address,
) -> TaxCalculationResult {
    // Check if pool is in address balance changes
    let pool_changes = match address_balance_changes.get(pool_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Pool address {} not found in address balance changes", pool_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Check if buyer is in address balance changes
    let buyer_changes = match address_balance_changes.get(buyer_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Buyer address {} not found in address balance changes", buyer_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Get the checksummed token address string for lookup
    let token_addr_str = to_checksum_address(token_address);
    
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
    if *pool_token_change >= I256::ZERO {
        let reason = format!("Pool token change is positive or zero ({}) - pool should lose tokens in a buy", 
               pool_token_change);
        warn!("INVALID_SIMULATION_RESULTS: {}", reason);
        return TaxCalculationResult::InvalidSimulation { reason };
    }
    let tokens_from_pool = pool_token_change.abs();
    
    // Get tokens received by buyer (positive value in token_net)
    let tokens_to_buyer = match buyer_changes.token_net.get(&token_addr_str) {
        Some(change) => *change,
        None => {
            let reason = format!("Token {} not found in buyer's token_net. Available tokens: {:?}", 
                   token_addr_str, buyer_changes.token_net.keys().collect::<Vec<_>>());
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Check if buyer received any tokens
    if tokens_to_buyer == I256::ZERO {
        // This is 100% tax (honeypot) - buyer got nothing
        debug!("Buy tax calc: Buyer received 0 tokens while pool sent {} - this is 100% tax", tokens_from_pool);
        return TaxCalculationResult::Calculated(100.0);
    }
    
    // Calculate tax percentage
    // Convert I256 to f64 for percentage calculation
    if tokens_from_pool > I256::ZERO {
        // Convert I256 to string then parse to f64 (safe for large values)
        let pool_amount: f64 = tokens_from_pool.to_string().parse().unwrap_or(0.0);
        let buyer_amount: f64 = tokens_to_buyer.to_string().parse().unwrap_or(0.0);
        let tax_percent: f64 = (1.0 - (buyer_amount / pool_amount)) * 100.0;
        TaxCalculationResult::Calculated(tax_percent.max(0.0))
    } else {
        let reason = "Pool sent zero tokens - invalid simulation state".to_string();
        warn!("INVALID_SIMULATION_RESULTS: {}", reason);
        TaxCalculationResult::InvalidSimulation { reason }
    }
}

/// Calculate sell tax from address balance changes
/// 
/// Sell Tax Formula: (1 - eth_received_by_seller / eth_sent_by_pool) × 100
/// 
/// We track:
/// 1. How much ETH leaves the pool (negative eth_net for pool)
/// 2. How much ETH the seller receives (positive eth_net for seller)
/// 3. The difference is the tax
/// 
/// Example: Pool sends 1 ETH, seller receives 0.9 ETH = 10% sell tax
pub fn calculate_sell_tax(
    address_balance_changes: &HashMap<Address, AddressBalanceChange>,
    pool_address: &Address,
    seller_address: &Address,
) -> TaxCalculationResult {
    // Check if pool is in address balance changes
    let pool_changes = match address_balance_changes.get(pool_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Pool address {} not found in address balance changes", pool_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Check if seller is in address balance changes
    let seller_changes = match address_balance_changes.get(seller_address) {
        Some(changes) => changes,
        None => {
            let reason = format!("Seller address {} not found in address balance changes", seller_address);
            warn!("INVALID_SIMULATION_RESULTS: {}", reason);
            return TaxCalculationResult::InvalidSimulation { reason };
        }
    };
    
    // Get ETH sent by pool (negative value in eth_net)
    let pool_eth_change = pool_changes.eth_net;
    
    // Pool loses ETH, so the value should be negative
    // We need the absolute value
    if pool_eth_change >= I256::ZERO {
        let reason = format!("Pool ETH change is positive or zero ({}) - pool should lose ETH in a sell", 
               pool_eth_change);
        warn!("INVALID_SIMULATION_RESULTS: {}", reason);
        return TaxCalculationResult::InvalidSimulation { reason };
    }
    let eth_from_pool = pool_eth_change.abs();
    
    // Get ETH received by seller (positive value in eth_net)
    let eth_to_seller = seller_changes.eth_net;
    
    // Check if seller received any ETH
    if eth_to_seller == I256::ZERO {
        // This is 100% tax (honeypot) - seller got nothing
        debug!("Sell tax calc: Seller received 0 ETH while pool sent {} - this is 100% tax", eth_from_pool);
        return TaxCalculationResult::Calculated(100.0);
    }
    
    // Calculate tax percentage
    // Convert I256 to f64 for percentage calculation
    if eth_from_pool > I256::ZERO {
        let pool_amount: f64 = eth_from_pool.to_string().parse().unwrap_or(0.0);
        let seller_amount: f64 = eth_to_seller.to_string().parse().unwrap_or(0.0);
        let tax_percent: f64 = (1.0 - (seller_amount / pool_amount)) * 100.0;
        TaxCalculationResult::Calculated(tax_percent.max(0.0))
    } else {
        let reason = "Pool sent zero ETH - invalid simulation state".to_string();
        warn!("INVALID_SIMULATION_RESULTS: {}", reason);
        TaxCalculationResult::InvalidSimulation { reason }
    }
}