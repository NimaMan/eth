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
pub mod entities;

// Legacy modules (to be refactored)
pub mod query_engine;

// Utility modules
pub mod postgres_db;
pub mod time_utils;
pub mod common_addresses;

// Re-export new provider architecture
pub use provider::{
    RethQueryProvider, Account, Portfolio, TokenMetadata, CompleteBalances, BalanceChanges,
    BlockHeader, TransactionData, TransactionReceipt, // FullTransactionData, BlockTransactions, // TEMPORARILY DISABLED
    // Types from address_state are exported via provider::*
};

// Re-export entity analysis types
pub use entities::{
    EntityType,
    StablecoinMarketData, StablecoinMarketAnalysis, MarketByUnitAnalysis, UnitMarketData,
    ExchangeBalance, CexBalanceSummary,
    ProviderHoldings, EtfHoldingsSummary,
};

// Re-export legacy types (to be deprecated)
pub use query_engine::ChainQuery;

// Re-export time utilities
pub use time_utils::{
    BlockTimeConverter, BlockTimestamp,
    TimePeriod, PeriodType, PeriodBoundary,
    TimestampCache,
};

// Re-export common addresses
pub use common_addresses::{
    DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS, ADDRESSES_BY_NAME, FEE_RECIPIENTS,
    // Address types
    stablecoins::{StablecoinInfo, STABLECOINS},
    cex::{CexAddress, CEX_ADDRESSES},
    etf::{EtfAddress, ETF_ADDRESSES},
};

// Re-export commonly used types from dependencies
pub use alloy_primitives::{Address, U256, B256};
pub use eyre::Result;
pub use tx_simulator::TxSimulator;