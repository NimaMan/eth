/// ETF (Exchange-Traded Fund) analysis module
/// 
/// Provides analysis for ETF provider addresses including:
/// - Holdings tracking
/// - Inflow/outflow analysis
/// - Provider comparison

pub mod addresses;
pub mod holdings_tracker;
pub mod flow_analysis;

// Re-export address data and utilities
pub use addresses::{
    EtfAddress, ETF_ADDRESSES, ETF_ADDRESS_COUNT,
    ETF_ADDRESS_SET, ETF_BY_ADDRESS, ADDRESSES_BY_PROVIDER,
    is_etf_address, get_etf_by_address, get_provider_addresses,
    provider_stats
};

// Re-export analysis functions
pub use holdings_tracker::EtfHoldingsTracker;
pub use flow_analysis::EtfFlowAnalyzer;