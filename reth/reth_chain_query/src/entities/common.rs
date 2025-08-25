/// Common utilities for entity analysis
/// 
/// Shared functionality used across all entity types.

pub mod aggregation;

use alloy_primitives::{Address, U256};
use std::collections::HashMap;

// Re-export aggregation types for convenience
pub use aggregation::{TimePeriod, PeriodFlowData, FlowStatistics};

/// Format token amount with proper decimals for display
pub fn format_token_amount(amount: U256, decimals: u8) -> f64 {
    let divisor = U256::from(10).pow(U256::from(decimals));
    let whole = amount / divisor;
    let fraction = amount % divisor;
    
    let whole_f64 = whole.to_string().parse::<f64>().unwrap_or(0.0);
    let fraction_f64 = fraction.to_string().parse::<f64>().unwrap_or(0.0) / 10_f64.powi(decimals as i32);
    
    whole_f64 + fraction_f64
}

/// Entity type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Stablecoin,
    CEX,
    ETF,
    Unknown,
}

/// Identify entity type for an address
pub fn identify_entity_type(address: Address) -> EntityType {
    if super::stablecoins::is_stablecoin(address) {
        EntityType::Stablecoin
    } else if super::cex::is_cex_address(address) {
        EntityType::CEX
    } else if super::etfs::is_etf_address(address) {
        EntityType::ETF
    } else {
        EntityType::Unknown
    }
}

/// Flow direction for transfers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    Inflow,
    Outflow,
    Internal, // Between addresses of same entity
}

/// Categorize flow between addresses
pub fn categorize_flow(from: Address, to: Address, entity_addresses: &[Address]) -> FlowDirection {
    let from_is_entity = entity_addresses.contains(&from);
    let to_is_entity = entity_addresses.contains(&to);
    
    match (from_is_entity, to_is_entity) {
        (true, true) => FlowDirection::Internal,
        (true, false) => FlowDirection::Outflow,
        (false, true) => FlowDirection::Inflow,
        (false, false) => FlowDirection::Internal, // Neither is entity
    }
}

/// Balance change summary
#[derive(Debug, Clone)]
pub struct BalanceChange {
    pub address: Address,
    pub old_balance: U256,
    pub new_balance: U256,
    pub change: i128,
    pub percent_change: f64,
}

impl BalanceChange {
    pub fn calculate(address: Address, old_balance: U256, new_balance: U256) -> Self {
        let change = if new_balance >= old_balance {
            (new_balance - old_balance).to_string().parse::<i128>().unwrap_or(i128::MAX)
        } else {
            -(old_balance - new_balance).to_string().parse::<i128>().unwrap_or(i128::MIN)
        };
        
        let percent_change = if old_balance > U256::ZERO {
            let old_f64 = old_balance.to_string().parse::<f64>().unwrap_or(0.0);
            let new_f64 = new_balance.to_string().parse::<f64>().unwrap_or(0.0);
            ((new_f64 - old_f64) / old_f64) * 100.0
        } else if new_balance > U256::ZERO {
            100.0
        } else {
            0.0
        };
        
        Self {
            address,
            old_balance,
            new_balance,
            change,
            percent_change,
        }
    }
}