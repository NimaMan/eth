use crate::tx_processor::address_balance_change_calculator::get_token_symbol;
use crate::tx_processor::data_models::ProcessedTransaction;
/// Tax Calculator for ERC20 Tokens using ProcessedTransaction
///
/// Uses the modern currency_net approach from ProcessedTransaction
/// to accurately calculate taxes from address balance changes.
use alloy_primitives::{Address, I256, U256};
use reth_chain_query::to_checksum_address;

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
fn extract_token_change(
    processed_tx: &ProcessedTransaction,
    address: Address,
    token_address: Address,
) -> Option<I256> {
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
        None => {
            return TaxCalculationResult::InvalidSimulation {
                reason: "Pool has no token balance change".to_string(),
            }
        }
    };

    let buyer_change = match buyer_token_change {
        Some(change) => change,
        None => {
            return TaxCalculationResult::InvalidSimulation {
                reason: "Buyer has no token balance change".to_string(),
            }
        }
    };

    // Pool should have negative change (sending tokens)
    if !pool_change.is_negative() {
        return TaxCalculationResult::InvalidSimulation {
            reason: format!(
                "Pool token change is non-negative ({}) - pool should lose tokens in a buy",
                pool_change
            ),
        };
    }

    // Buyer should have positive change (receiving tokens)
    if buyer_change.is_negative() {
        return TaxCalculationResult::InvalidSimulation {
            reason: "Buyer has negative token change - should gain tokens in buy".to_string(),
        };
    }

    // Convert pool change to positive (tokens sent out)
    let pool_tokens_sent = pool_change.unsigned_abs();
    let buyer_tokens_received = buyer_change.unsigned_abs();

    // Calculate tax: tokens sent by pool - tokens received by buyer
    if pool_tokens_sent < buyer_tokens_received {
        // This shouldn't happen - buyer can't receive more than pool sent
        return TaxCalculationResult::InvalidSimulation {
            reason: "Buyer received more tokens than pool sent".to_string(),
        };
    }

    let tax_amount = pool_tokens_sent - buyer_tokens_received;
    // Avoid division by zero: if pool sent zero tokens, tax is zero
    if pool_tokens_sent == U256::ZERO {
        return TaxCalculationResult::Calculated {
            tax_basis_points: 0,
        };
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

/// Calculate sell tax from ProcessedTransaction using address balance changes
///
/// Sell Tax Logic:
/// 1. Seller should have negative token change (sends tokens)
/// 2. Pool should have positive token change (receives tokens)
/// 3. Identify the sold token by matching seller's negative change with pool's positive change
/// 4. Tax = tokens_sent_by_seller - tokens_received_by_pool
/// 5. Tax percentage = (tax_amount * 10000) / tokens_sent_by_seller (basis points)
pub fn calculate_sell_tax_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    pool_address: Address,
    seller_address: Address,
) -> TaxCalculationResult {
    // Fetch balance changes for seller and pool
    let seller_changes = match processed_tx.address_balance_changes.get(&seller_address) {
        Some(changes) => changes,
        None => {
            return TaxCalculationResult::InvalidSimulation {
                reason: "Seller has no balance changes".to_string(),
            }
        }
    };
    let pool_changes = match processed_tx.address_balance_changes.get(&pool_address) {
        Some(changes) => changes,
        None => {
            return TaxCalculationResult::InvalidSimulation {
                reason: "Pool has no balance changes".to_string(),
            }
        }
    };

    // Try to find the sold token in token_net first (unknown tokens),
    // then fallback to currency_net (known tokens like USDC/USDT/WETH but not ETH for gas).
    let mut best_match: Option<(I256, I256)> = None; // (seller_negative, pool_positive)

    // token_net path: keys are checksum addresses
    for (token_key, &seller_amount) in &seller_changes.token_net {
        if !seller_amount.is_negative() {
            continue; // seller must be sending tokens
        }
        if let Some(&pool_amount) = pool_changes.token_net.get(token_key) {
            if pool_amount > I256::ZERO {
                // Choose the largest absolute seller amount if multiple candidates
                let pick = match best_match {
                    None => true,
                    Some((prev_seller_amt, _)) => {
                        seller_amount.unsigned_abs() > prev_seller_amt.unsigned_abs()
                    }
                };
                if pick {
                    best_match = Some((seller_amount, pool_amount));
                }
            }
        }
    }

    // currency_net path: keys are symbols (exclude ETH since gas can make it negative)
    for (symbol, &seller_amount) in &seller_changes.currency_net {
        if *symbol == "ETH" || !seller_amount.is_negative() {
            continue;
        }
        if let Some(&pool_amount) = pool_changes.currency_net.get(symbol) {
            if pool_amount > I256::ZERO {
                let pick = match best_match {
                    None => true,
                    Some((prev_seller_amt, _)) => {
                        seller_amount.unsigned_abs() > prev_seller_amt.unsigned_abs()
                    }
                };
                if pick {
                    best_match = Some((seller_amount, pool_amount));
                }
            }
        }
    }

    let (seller_change, pool_change) = match best_match {
        Some(pair) => pair,
        None => {
            return TaxCalculationResult::InvalidSimulation {
                reason: "Could not identify sold token from balance changes".to_string(),
            }
        }
    };

    // Sanity: seller must send (negative), pool must receive (positive)
    if !seller_change.is_negative() {
        return TaxCalculationResult::InvalidSimulation {
            reason: "Seller token change is non-negative - should send tokens in sell".to_string(),
        };
    }
    if pool_change.is_negative() || pool_change == I256::ZERO {
        return TaxCalculationResult::InvalidSimulation {
            reason: "Pool token change is non-positive - should receive tokens in sell".to_string(),
        };
    }

    let seller_tokens_sent = seller_change.unsigned_abs();
    let pool_tokens_received = pool_change.unsigned_abs();

    if pool_tokens_received > seller_tokens_sent {
        return TaxCalculationResult::InvalidSimulation {
            reason: "Pool received more tokens than seller sent".to_string(),
        };
    }

    let tax_amount = seller_tokens_sent - pool_tokens_received;
    if seller_tokens_sent == U256::ZERO {
        return TaxCalculationResult::Calculated {
            tax_basis_points: 0,
        };
    }

    let tax_bps_u256 = (tax_amount * U256::from(10000)) / seller_tokens_sent;
    let tax_basis_points: u32 = tax_bps_u256.try_into().unwrap_or(u32::MAX);
    TaxCalculationResult::Calculated { tax_basis_points }
}
