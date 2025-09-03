/// Entity tracking and analysis module
/// 
/// Provides comprehensive analysis for major blockchain entities:
/// - Stablecoins: Market share, supply tracking, concentration metrics
/// - CEX (Centralized Exchanges): Balance tracking, flow analysis
/// - ETF (Exchange-Traded Funds): Holdings tracking, provider analysis

pub mod types;
pub mod stablecoins;
pub mod cex;
pub mod etf;

// Re-export common types
pub use types::{EntityType, format_token_amount};

// Re-export stablecoin types
pub use stablecoins::{
    StablecoinMarketData, StablecoinMarketAnalysis, 
    MarketByUnitAnalysis, UnitMarketData,
};

// Re-export CEX types
pub use cex::{ExchangeBalance, CexBalanceSummary};

// Re-export ETF types
pub use etf::{ProviderHoldings, EtfHoldingsSummary};

use alloy_primitives::Address;
use crate::provider::RethQueryProvider;
use crate::common_addresses::{
    stablecoins::get_stablecoin_by_address,
    cex::get_cex_by_address,
    etf::get_etf_by_address,
};

impl RethQueryProvider {
    /// Identify entity type for an address
    pub fn identify_entity_type(&self, address: Address) -> EntityType {
        if get_stablecoin_by_address(address).is_some() {
            EntityType::Stablecoin
        } else if get_cex_by_address(address).is_some() {
            EntityType::CEX
        } else if get_etf_by_address(address).is_some() {
            EntityType::ETF
        } else {
            EntityType::Unknown
        }
    }
}