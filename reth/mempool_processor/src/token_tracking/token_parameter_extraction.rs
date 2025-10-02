//! Token parameter helpers.
//!
//! Historical tax-calculation code lived here, but all simulation logic now
//! lives in `tx_processor`. We keep a thin wrapper so consumers inside the
//! mempool processor can fetch ERC20 metadata without re-implementing ABI
//! calls. Everything delegates to the canonical helpers in `reth_chain_query`
//! to avoid divergent behavior across crates.

use alloy_primitives::{Address, U256};
use eyre::Result;

use reth_chain_query::provider::RethQueryProvider;
pub use reth_chain_query::provider::TokenMetadata;

/// Fetch full ERC20 token metadata (name, symbol, decimals, total supply)
/// via the shared `RethQueryProvider`.
pub async fn fetch_token_metadata(
    provider: &RethQueryProvider,
    token: Address,
) -> Result<TokenMetadata> {
    provider.get_token_metadata(token).await
}

/// Fetch the token's decimals at an optional historical block.
pub async fn fetch_token_decimals(
    provider: &RethQueryProvider,
    token: Address,
    block: Option<u64>,
) -> Result<u8> {
    provider.get_token_decimals(token, block).await
}

/// Fetch the token's symbol at an optional historical block.
pub async fn fetch_token_symbol(
    provider: &RethQueryProvider,
    token: Address,
    block: Option<u64>,
) -> Result<String> {
    provider.get_token_symbol(token, block).await
}

/// Fetch the token's name at an optional historical block.
pub async fn fetch_token_name(
    provider: &RethQueryProvider,
    token: Address,
    block: Option<u64>,
) -> Result<String> {
    provider.get_token_name(token, block).await
}

/// Fetch the token's total supply at an optional historical block.
pub async fn fetch_token_total_supply(
    provider: &RethQueryProvider,
    token: Address,
    block: Option<u64>,
) -> Result<U256> {
    provider.get_token_total_supply(token, block).await
}
