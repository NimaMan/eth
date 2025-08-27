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

pub mod account;
pub mod token;
pub mod storage;
pub mod block;
pub mod query_engine;
pub mod postgres_db;
pub mod entities;
pub mod time_utils;
pub mod common_addresses;

// Re-export main types
pub use query_engine::ChainQuery;
pub use account::{AccountInfo, AccountQuery};
pub use token::{TokenQuery, ERC20Info};
pub use storage::StorageQuery;
pub use block::{BlockInfo, BlockQuery};

// Re-export entity analysis
pub use entities::{
    stablecoins::{StablecoinInfo, STABLECOINS, StablecoinMarketAnalyzer, StablecoinSupplyTracker},
    cex::{CexAddress, CEX_ADDRESSES, CexBalanceTracker, CexFlowAnalyzer},
    etfs::{EtfAddress, ETF_ADDRESSES, EtfHoldingsTracker, EtfFlowAnalyzer},
};

// Re-export time utilities
pub use time_utils::{
    BlockTimeConverter, BlockTimestamp,
    TimePeriod, PeriodType, PeriodBoundary,
    TimestampCache,
};

// Re-export common addresses
pub use common_addresses::{DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS, ADDRESSES_BY_NAME};

// Re-export commonly used types from dependencies
pub use alloy_primitives::{Address, U256, B256};
pub use eyre::Result;