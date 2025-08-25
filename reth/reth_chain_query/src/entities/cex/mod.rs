/// CEX (Centralized Exchange) analysis module
/// 
/// Provides analysis for centralized exchange addresses including:
/// - Balance tracking across all CEX addresses
/// - Flow analysis (deposits/withdrawals)
/// - Exchange comparison and rankings

pub mod addresses;
pub mod balance_tracker;
pub mod flow_analysis;

// Re-export address data and utilities
pub use addresses::{
    CexAddress, CEX_ADDRESSES, CEX_ADDRESS_COUNT,
    CEX_ADDRESS_SET, CEX_BY_ADDRESS, ADDRESSES_BY_EXCHANGE,
    is_cex_address, get_cex_by_address, get_exchange_addresses,
    exchange_stats
};

// Re-export analysis functions
pub use balance_tracker::CexBalanceTracker;
pub use flow_analysis::CexFlowAnalyzer;