pub mod contracts;
pub mod entities;
pub mod live_chain;
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
pub use utils::time_utils::{
    BlockTimeConverter, BlockTimestamp, PeriodBoundary, PeriodType, TimePeriod, TimestampCache,
};

// Re-export DEX helpers
pub use dex::*;

// Re-export common addresses
pub use common_addresses::{
    identify_known_address, is_known_address, CexAddress, EtfAddress, KnownAddress,
    KnownAddressKind, StablecoinInfo, ADDRESSES_BY_NAME, CEX_ADDRESSES, DENOM_ADDRESSES,
    ERC20_TOKEN_DECIMALS, ETF_ADDRESSES, FEE_RECIPIENTS, STABLECOINS,
};

// Re-export commonly used types from dependencies
pub use alloy_primitives::{Address, B256, U256};
pub use eyre::Result;
pub use tx_simulator::TxSimulator;
// Re-export checksum utilities for convenience
pub use utils::checksum::{
    alloy_address_to_checksum, deserialize_address_checksum, serialize_address_checksum,
    to_checksum_address,
};
