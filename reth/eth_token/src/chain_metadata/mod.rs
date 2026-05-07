//! Chain-backed metadata providers used by token indexing pipelines.

mod reth;
mod static_provider;
mod types;

pub use reth::{LiveRethChainMetadataProvider, RethChainMetadataProvider};
pub use static_provider::{
    NoopUniswapV2PoolMetadataProvider, StaticTokenMetadataProvider,
    StaticUniswapV2PoolMetadataProvider,
};
pub use types::{
    TokenDiscoveryProvider, TokenMetadataLookup, TokenMetadataProvider, UniswapV2PoolMetadata,
    UniswapV2PoolMetadataLookup, UniswapV2PoolMetadataProvider,
};
