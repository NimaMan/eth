/// Tax Calculator for ERC20 Tokens using ProcessedTransaction
/// 
/// Uses the modern currency_net approach from ProcessedTransaction
/// to accurately calculate taxes from address balance changes.

use alloy_primitives::{Address, U256};
use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::address_balance_change_calculator::get_token_symbol;
use crate::utils::to_checksum_address;

/// Check if U256 value represents a negative number (two's complement)
fn is_negative_u256(value: U256) -> bool {
    value > U256::MAX / U256::from(2)
}

/// Get absolute value of potentially negative U256 (two's complement)
fn abs_u256(value: U256) -> U256 {
    if is_negative_u256(value) {
        // Negative value - convert to positive
        U256::MAX - value + U256::from(1)
    } else {
        // Already positive
        value
    }
}

/// Result of tax calculation
#[derive(Debug, Clone)]
pub enum TaxCalculationResult {
    /// Tax calculated successfully in basis points (e.g., 30 = 0.30%)
    Calculated { tax_basis_points: u32 },
    /// Unable to calculate due to invalid simulation data
    InvalidSimulation { reason: String },
}

impl TaxCalculationResult {
    /// Get tax percentage as f64 for display (e.g., 30 basis points = 0.30%)
    pub fn as_percentage(&self) -> Option<f64> {
        match self {
            TaxCalculationResult::Calculated { tax_basis_points } => {
                Some(*tax_basis_points as f64 / 100.0)
            }
            TaxCalculationResult::InvalidSimulation { .. } => None,
        }
    }
    
    /// Check if calculation was successful
    pub fn is_success(&self) -> bool {
        matches!(self, TaxCalculationResult::Calculated { .. })
    }
}

/// Extract token change for an address from address_balance_changes
/// Returns the raw U256 value (potentially negative in two's complement)
fn extract_token_change(processed_tx: &ProcessedTransaction, address: Address, token_address: Address) -> Option<U256> {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&address) {
        // Check if it's a known token (in currency_net)
        if let Some(symbol) = get_token_symbol(&token_address) {
            if let Some(&amount) = balance_changes.currency_net.get(symbol) {
                return Some(amount);
            }
        } else {
            // Unknown token - check token_net
            let token_key = to_checksum_address(&token_address);
            if let Some(&amount) = balance_changes.token_net.get(&token_key) {
                return Some(amount);
            }
        }
    }
    None
}

/// Calculate buy tax from ProcessedTransaction using address balance changes
/// 
/// Buy Tax Logic:
/// 1. Pool should have negative token change (loses tokens)
/// 2. Buyer should have positive token change (gains tokens)  
/// 3. Tax = tokens_sent_by_pool - tokens_received_by_buyer
/// 4. Tax percentage = (tax_amount * 10000) / tokens_sent_by_pool (basis points)
pub fn calculate_buy_tax_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    pool_address: Address,
    buyer_address: Address,
    token_address: Address,
) -> TaxCalculationResult {
    // Get token changes from address_balance_changes
    let pool_token_change = extract_token_change(processed_tx, pool_address, token_address);
    let buyer_token_change = extract_token_change(processed_tx, buyer_address, token_address);
    
    let pool_change = match pool_token_change {
        Some(change) => change,
        None => return TaxCalculationResult::InvalidSimulation {
            reason: "Pool has no token balance change".to_string()
        }
    };
    
    let buyer_change = match buyer_token_change {
        Some(change) => change,
        None => return TaxCalculationResult::InvalidSimulation {
            reason: "Buyer has no token balance change".to_string()
        }
    };
    
    // Pool should have negative change (sending tokens)
    if !is_negative_u256(pool_change) {
        return TaxCalculationResult::InvalidSimulation {
            reason: format!("Pool token change is non-negative ({}) - pool should lose tokens in a buy", pool_change)
        };
    }
    
    // Buyer should have positive change (receiving tokens)
    if is_negative_u256(buyer_change) {
        return TaxCalculationResult::InvalidSimulation {
            reason: "Buyer has negative token change - should gain tokens in buy".to_string()
        };
    }
    
    // Convert pool change to positive (tokens sent out)
    let pool_tokens_sent = abs_u256(pool_change);
    let buyer_tokens_received = buyer_change;
    
    // Calculate tax: tokens sent by pool - tokens received by buyer
    if pool_tokens_sent < buyer_tokens_received {
        // This shouldn't happen - buyer can't receive more than pool sent
        return TaxCalculationResult::InvalidSimulation {
            reason: "Buyer received more tokens than pool sent".to_string()
        };
    }
    
    let tax_amount = pool_tokens_sent - buyer_tokens_received;
    
    // Calculate tax percentage in basis points (e.g., 30 = 0.30%)
    if pool_tokens_sent == U256::ZERO {
        return TaxCalculationResult::Calculated { tax_basis_points: 0 };
    }
    
    // tax_basis_points = (tax_amount * 10000) / pool_tokens_sent
    let tax_basis_points = if tax_amount == U256::ZERO {
        0
    } else {
        let basis_points_calc = (tax_amount * U256::from(10000)) / pool_tokens_sent;
        // Convert to u32, capping at u32::MAX if somehow larger
        basis_points_calc.try_into().unwrap_or(u32::MAX)
    };
    
    TaxCalculationResult::Calculated { tax_basis_points }
}

// TODO: Implement sell tax calculation with U256 arithmetic if needed