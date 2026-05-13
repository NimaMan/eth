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

pub use crate::common_addresses::{KnownAddress, KnownAddressKind};

use crate::provider::RethQueryProvider;
use alloy_primitives::Address;

impl RethQueryProvider {
    /// Identify a known important address using the canonical common-address catalogs.
    pub fn identify_known_address(&self, address: Address) -> Option<KnownAddress> {
        crate::common_addresses::identify_known_address(address)
    }

    /// Identify entity type for an address
    pub fn identify_entity_type(&self, address: Address) -> EntityType {
        match self.identify_known_address(address).map(|known| known.kind) {
            Some(KnownAddressKind::Stablecoin) => EntityType::Stablecoin,
            Some(KnownAddressKind::Cex) => EntityType::CEX,
            Some(KnownAddressKind::Etf) => EntityType::ETF,
            _ => EntityType::Unknown,
        }
    }
}
