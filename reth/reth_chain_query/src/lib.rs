pub mod entities;
/// Reth Chain Query Library
///
/// Direct blockchain state queries using Reth's local database.
/// Provides 5-10x faster queries compared to RPC by reading directly from disk.
///
/// # Architecture
///
/// This library provides a clean interface for querying blockchain state:
/// - Account queries (balance, nonce, code)
/// - Token queries (ERC20/721/1155 data)
/// - Storage queries (direct slot access)
/// - Block queries (timestamps, hashes)
///
/// All queries bypass RPC and read directly from Reth's MDBX database.
// Core modules
pub mod provider;
pub mod reth_index;

// Legacy modules (to be refactored)
pub mod query_engine;

// Utility modules
pub mod common_addresses;
pub mod dex;
pub mod postgres_db;
pub mod time_utils;
pub mod tx_builders;
pub mod utils;

pub use utils::function_signatures;

// Re-export new provider architecture
pub use provider::provider_factory_from_datadir;
pub use provider::{
    Account,
    BalanceChanges,
    BlockHeader,
    CompleteBalances,
    Portfolio,
    RethQueryProvider,
    TokenMetadata,
    TransactionData,
    TransactionReceipt, // FullTransactionData, BlockTransactions, // TEMPORARILY DISABLED
                        // Types from address_state are exported via provider::*
};

// Re-export entity analysis types
pub use entities::{
    CexBalanceSummary, EntityType, EtfHoldingsSummary, ExchangeBalance, MarketByUnitAnalysis,
    ProviderHoldings, StablecoinMarketAnalysis, StablecoinMarketData, UnitMarketData,
};

// Re-export legacy types (to be deprecated)
pub use query_engine::ChainQuery;

// Re-export time utilities
pub use time_utils::{
    BlockTimeConverter, BlockTimestamp, PeriodBoundary, PeriodType, TimePeriod, TimestampCache,
};

// Re-export DEX helpers
pub use dex::{
    balancer::*, curve::*, pool_types::*, sushiswap::*, uniswap_v2::*, uniswap_v3::*, uniswap_v4::*,
};

// Re-export common addresses
pub use common_addresses::{
    cex::{CexAddress, CEX_ADDRESSES},
    etf::{EtfAddress, ETF_ADDRESSES},
    // Address types
    stablecoins::{StablecoinInfo, STABLECOINS},
    ADDRESSES_BY_NAME,
    DENOM_ADDRESSES,
    ERC20_TOKEN_DECIMALS,
    FEE_RECIPIENTS,
};

// Re-export swap route for convenience
pub use tx_builders::amm_swap_route::AmmSwapRoute;

// Re-export commonly used types from dependencies
pub use alloy_primitives::{Address, B256, U256};
pub use eyre::Result;
pub use tx_simulator::TxSimulator;
// Re-export checksum utilities for convenience
pub use utils::checksum::{
    alloy_address_to_checksum, deserialize_address_checksum, serialize_address_checksum,
    to_checksum_address,
};
