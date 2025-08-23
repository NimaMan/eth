/// Tax calculation for buy and sell transactions

use alloy_primitives::{Address, I256, U256};
use std::collections::HashMap;
use reth_tx_simulator::AddressBalanceChange;

pub enum TaxCalculationResult {
    Calculated(f64),
    InvalidSimulation { reason: String },
}

/// Calculate buy tax from address balance changes
/// Buy tax = (ETH spent - token value received) / ETH spent * 100
pub fn calculate_buy_tax(
    address_balance_changes: &HashMap<Address, AddressBalanceChange>,
    pool_address: &Address,
    buyer_address: &Address,
    _token_address: &Address, // Prefix with underscore to indicate intentionally unused
) -> TaxCalculationResult {
    // Get buyer's balance changes
    let buyer_state = match address_balance_changes.get(buyer_address) {
        Some(state) => state,
        None => return TaxCalculationResult::InvalidSimulation {
            reason: "No balance changes for buyer".to_string()
        },
    };
    
    // Get pool's balance changes
    let pool_state = match address_balance_changes.get(pool_address) {
        Some(state) => state,
        None => return TaxCalculationResult::InvalidSimulation {
            reason: "No balance changes for pool".to_string()
        },
    };
    
    // ETH spent by buyer (should be negative)
    let eth_spent = buyer_state.eth_net;
    if eth_spent >= I256::ZERO {
        return TaxCalculationResult::InvalidSimulation {
            reason: "Buyer didn't spend ETH".to_string()
        };
    }
    
    // ETH received by pool (should be positive)
    let eth_to_pool = pool_state.eth_net;
    if eth_to_pool <= I256::ZERO {
        return TaxCalculationResult::InvalidSimulation {
            reason: "Pool didn't receive ETH".to_string()
        };
    }
    
    // Calculate tax as percentage
    // Tax = (ETH spent - ETH to pool) / ETH spent * 100
    // Note: eth_spent is negative, so we negate it
    let spent_abs = eth_spent.abs();
    let tax_amount = spent_abs - eth_to_pool;
    
    if spent_abs > I256::ZERO {
        // Convert I256 to f64 for calculation
        let tax_amount_f64 = if tax_amount >= I256::ZERO {
            tax_amount.unsigned_abs().to_string().parse::<f64>().unwrap_or(0.0)
        } else {
            -(tax_amount.abs().unsigned_abs().to_string().parse::<f64>().unwrap_or(0.0))
        };
        let spent_abs_f64 = spent_abs.unsigned_abs().to_string().parse::<f64>().unwrap_or(1.0);
        let tax_percent = (tax_amount_f64 / spent_abs_f64) * 100.0;
        TaxCalculationResult::Calculated(tax_percent)
    } else {
        TaxCalculationResult::InvalidSimulation {
            reason: "Invalid ETH amount".to_string()
        }
    }
}

/// Calculate sell tax from address balance changes
/// Sell tax = (tokens sent - ETH value received) / tokens sent * 100
pub fn calculate_sell_tax(
    address_balance_changes: &HashMap<Address, AddressBalanceChange>,
    pool_address: &Address,
    buyer_address: &Address,
) -> TaxCalculationResult {
    // Get buyer's balance changes
    let buyer_state = match address_balance_changes.get(buyer_address) {
        Some(state) => state,
        None => return TaxCalculationResult::InvalidSimulation {
            reason: "No balance changes for buyer".to_string()
        },
    };
    
    // Get pool's balance changes
    let pool_state = match address_balance_changes.get(pool_address) {
        Some(state) => state,
        None => return TaxCalculationResult::InvalidSimulation {
            reason: "No balance changes for pool".to_string()
        },
    };
    
    // ETH received by buyer (should be positive)
    let eth_received = buyer_state.eth_net;
    if eth_received <= I256::ZERO {
        return TaxCalculationResult::InvalidSimulation {
            reason: "Buyer didn't receive ETH".to_string()
        };
    }
    
    // ETH sent from pool (should be negative)
    let eth_from_pool = pool_state.eth_net;
    if eth_from_pool >= I256::ZERO {
        return TaxCalculationResult::InvalidSimulation {
            reason: "Pool didn't send ETH".to_string()
        };
    }
    
    // Calculate tax as percentage
    // Tax = (ETH from pool - ETH received) / ETH from pool * 100
    // Note: eth_from_pool is negative, so we negate it
    let from_pool_abs = eth_from_pool.abs();
    let tax_amount = from_pool_abs - eth_received;
    
    if from_pool_abs > I256::ZERO {
        // Convert I256 to f64 for calculation
        let tax_amount_f64 = if tax_amount >= I256::ZERO {
            tax_amount.unsigned_abs().to_string().parse::<f64>().unwrap_or(0.0)
        } else {
            -(tax_amount.abs().unsigned_abs().to_string().parse::<f64>().unwrap_or(0.0))
        };
        let from_pool_abs_f64 = from_pool_abs.unsigned_abs().to_string().parse::<f64>().unwrap_or(1.0);
        let tax_percent = (tax_amount_f64 / from_pool_abs_f64) * 100.0;
        TaxCalculationResult::Calculated(tax_percent)
    } else {
        TaxCalculationResult::InvalidSimulation {
            reason: "Invalid ETH amount".to_string()
        }
    }
}

/// Extract ETH amount received from address balance changes
pub fn extract_eth_received(
    address_balance_changes: &HashMap<Address, AddressBalanceChange>,
    buyer_address: &Address,
) -> U256 {
    address_balance_changes.get(buyer_address)
        .and_then(|state| {
            if state.eth_net > I256::ZERO {
                Some(U256::try_from(state.eth_net.unsigned_abs()).unwrap_or(U256::ZERO))
            } else {
                None
            }
        })
        .unwrap_or(U256::ZERO)
}