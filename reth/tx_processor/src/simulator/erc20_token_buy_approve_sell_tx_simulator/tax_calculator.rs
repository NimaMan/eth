/// Tax calculation for buy and sell transactions
/// 
/// Uses the modern currency_net approach from ProcessedTransaction
/// to accurately calculate taxes from ETH balance changes.

use alloy_primitives::{Address, U256};
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::address_balance_change_calculator::get_token_symbol;
use crate::utils::to_checksum_address;

/// Result of tax calculation
#[derive(Debug, Clone)]
pub enum TaxCalculationResult {
    /// Tax calculated successfully
    Calculated { tax_percentage: f64 },
    /// Unable to calculate due to invalid simulation data
    InvalidSimulation { reason: String },
}

impl TaxCalculationResult {
    /// Extract tax percentage, or return default value if calculation failed
    pub fn tax_percentage_or(&self, default: f64) -> f64 {
        match self {
            TaxCalculationResult::Calculated { tax_percentage } => *tax_percentage,
            TaxCalculationResult::InvalidSimulation { .. } => default,
        }
    }
    
    /// Check if calculation was successful
    pub fn is_success(&self) -> bool {
        matches!(self, TaxCalculationResult::Calculated { .. })
    }
}

/// Extract ETH change for an address from address_balance_changes
fn extract_eth_change(processed_tx: &ProcessedTransaction, address: Address) -> f64 {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&address) {
        if let Some(&eth_amount) = balance_changes.currency_net.get("ETH") {
            // Convert from wei to ETH
            return eth_amount.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        }
    }
    0.0
}

/// Extract token change for an address from address_balance_changes
fn extract_token_change(processed_tx: &ProcessedTransaction, address: Address, token_address: Address) -> f64 {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&address) {
        // Check if it's a known token (in currency_net)
        if let Some(symbol) = get_token_symbol(&token_address) {
            if let Some(&amount) = balance_changes.currency_net.get(symbol) {
                // For known tokens, amount is already with decimals applied
                return amount.to_string().parse::<f64>().unwrap_or(0.0);
            }
        } else {
            // Unknown token - check token_net
            let token_key = to_checksum_address(&token_address);
            if let Some(&amount) = balance_changes.token_net.get(&token_key) {
                // Raw amount with decimals
                return amount.to_string().parse::<f64>().unwrap_or(0.0);
            }
        }
    }
    0.0
}

/// Calculate buy tax from ProcessedTransaction
/// 
/// Buy Tax Formula: (1 - tokens_received_by_buyer / tokens_sent_by_pool) × 100
/// 
/// We look at ERC20 transfers to determine:
/// 1. Total tokens sent from the pool
/// 2. Tokens received by the buyer
/// 3. The difference is the tax
pub fn calculate_buy_tax_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    pool_address: Address,
    buyer_address: Address,
    token_address: Address,
) -> TaxCalculationResult {
    // Get token changes from address_balance_changes
    let pool_token_change = extract_token_change(processed_tx, pool_address, token_address);
    let buyer_token_change = extract_token_change(processed_tx, buyer_address, token_address);
    
    // Pool should have negative change (sending tokens), buyer should have positive (receiving)
    if pool_token_change >= 0.0 {
        return TaxCalculationResult::InvalidSimulation {
            reason: format!("Pool token change is non-negative ({}) - pool should lose tokens in a buy", pool_token_change)
        };
    }
    
    if buyer_token_change <= 0.0 {
        // Buyer received no tokens - 100% tax (honeypot)
        if pool_token_change < 0.0 {
            return TaxCalculationResult::Calculated { 
                tax_percentage: 100.0
            };
        } else {
            return TaxCalculationResult::InvalidSimulation {
                reason: format!("Buyer token change is non-positive ({}) - invalid buy simulation", buyer_token_change)
            };
        }
    }
    
    // Calculate tax percentage
    // Pool loses tokens (negative), so we need absolute value
    let tokens_from_pool = pool_token_change.abs();
    let tokens_to_buyer = buyer_token_change;
    
    if tokens_from_pool > 0.0 {
        let tax_percent = (1.0 - (tokens_to_buyer / tokens_from_pool)) * 100.0;
        TaxCalculationResult::Calculated {
            tax_percentage: tax_percent.max(0.0)
        }
    } else {
        TaxCalculationResult::InvalidSimulation {
            reason: "Pool sent zero tokens - invalid calculation".to_string()
        }
    }
}

/// Calculate sell tax from ProcessedTransaction
/// 
/// Sell Tax Formula: (1 - eth_received_by_seller / eth_sent_by_pool) × 100
/// 
/// We look at internal transactions to determine:
/// 1. Total ETH sent from the pool
/// 2. ETH received by the seller
/// 3. The difference is the tax
pub fn calculate_sell_tax_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    pool_address: Address,
    seller_address: Address,
) -> TaxCalculationResult {
    // Get ETH changes from address_balance_changes
    let pool_eth_change = extract_eth_change(processed_tx, pool_address);
    let seller_eth_change = extract_eth_change(processed_tx, seller_address);
    
    // Pool should have negative change (sending ETH), seller should have positive (receiving)
    if pool_eth_change >= 0.0 {
        return TaxCalculationResult::InvalidSimulation {
            reason: format!("Pool ETH change is non-negative ({}) - pool should lose ETH in a sell", pool_eth_change)
        };
    }
    
    if seller_eth_change <= 0.0 {
        // Seller received no ETH - 100% tax (honeypot)
        if pool_eth_change < 0.0 {
            return TaxCalculationResult::Calculated {
                tax_percentage: 100.0
            };
        } else {
            return TaxCalculationResult::InvalidSimulation {
                reason: format!("Seller ETH change is non-positive ({}) - invalid sell simulation", seller_eth_change)
            };
        }
    }
    
    // Calculate tax percentage
    // Pool loses ETH (negative), so we need absolute value
    let eth_from_pool = pool_eth_change.abs();
    let eth_to_seller = seller_eth_change;
    
    if eth_from_pool > 0.0 {
        let tax_percent = (1.0 - (eth_to_seller / eth_from_pool)) * 100.0;
        TaxCalculationResult::Calculated {
            tax_percentage: tax_percent.max(0.0)
        }
    } else {
        TaxCalculationResult::InvalidSimulation {
            reason: "Pool sent zero ETH - invalid calculation".to_string()
        }
    }
}