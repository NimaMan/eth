/// Stablecoin analysis module
/// 
/// Provides comprehensive stablecoin analysis including:
/// - Market share calculations
/// - Supply tracking
/// - Whale concentration analysis
/// - Mint/burn detection

pub mod addresses;
pub mod market_share;
pub mod supply_analysis;

// Re-export address data and utilities
pub use addresses::{
    StablecoinInfo, STABLECOINS,
    STABLECOIN_BY_ADDRESS, STABLECOIN_BY_SYMBOL,
    is_stablecoin, get_stablecoin_by_address, get_stablecoin_by_symbol
};

// Re-export analysis functions
pub use market_share::StablecoinMarketAnalyzer;
pub use supply_analysis::StablecoinSupplyTracker;