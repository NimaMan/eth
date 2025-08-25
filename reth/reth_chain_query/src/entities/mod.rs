/// Entity analysis modules for major blockchain actors
/// 
/// This module provides comprehensive analysis for major entities on the blockchain:
/// - Stablecoins: Market share, supply tracking, whale analysis
/// - CEX: Balance tracking, flow analysis for centralized exchanges
/// - ETFs: Holdings tracking, inflow/outflow for ETF providers
/// 
/// All address data is imported from the Python eth_data module to maintain
/// consistency across the ecosystem.

pub mod common;
pub mod stablecoins;
pub mod cex;
pub mod etfs;

// Re-export main types for convenience
pub use stablecoins::{
    StablecoinInfo, STABLECOINS, 
    is_stablecoin, get_stablecoin_by_address, get_stablecoin_by_symbol
};

pub use cex::{
    CexAddress, CEX_ADDRESSES, CEX_ADDRESS_COUNT,
    is_cex_address, get_cex_by_address, get_exchange_addresses
};

pub use etfs::{
    EtfAddress, ETF_ADDRESSES, ETF_ADDRESS_COUNT,
    is_etf_address, get_etf_by_address, get_provider_addresses
};