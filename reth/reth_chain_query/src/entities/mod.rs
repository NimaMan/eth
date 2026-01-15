pub mod cex;
pub mod etf;
pub mod stablecoins;
/// Entity tracking and analysis module
///
/// Provides comprehensive analysis for major blockchain entities:
/// - Stablecoins: Market share, supply tracking, concentration metrics
/// - CEX (Centralized Exchanges): Balance tracking, flow analysis
/// - ETF (Exchange-Traded Funds): Holdings tracking, provider analysis
pub mod types;

// Re-export common types
pub use types::{format_token_amount, EntityType};

// Re-export stablecoin types
pub use stablecoins::{
    MarketByUnitAnalysis, StablecoinMarketAnalysis, StablecoinMarketData, UnitMarketData,
};

// Re-export CEX types
pub use cex::{CexBalanceSummary, ExchangeBalance};

// Re-export ETF types
pub use etf::{EtfHoldingsSummary, ProviderHoldings};

use crate::common_addresses::{
    cex::get_cex_by_address, etf::get_etf_by_address, stablecoins::get_stablecoin_by_address,
};
use crate::provider::RethQueryProvider;
use alloy_primitives::Address;

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
