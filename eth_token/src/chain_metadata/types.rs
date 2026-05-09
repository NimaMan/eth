use std::future::Future;
use std::pin::Pin;

use alloy_primitives::{Address, B256};
use eyre::Result;
use reth_chain_query::common_addresses::KnownV2Protocol;

use crate::erc20::ERC20TokenMetadata;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenMetadataLookup {
    pub token_address: Address,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub metadata_block_number: u64,
    pub transaction_hash: B256,
    pub tx_index: u64,
    pub creator_address: Address,
    pub creator_nonce: u64,
    pub pending_tx_hashes: Vec<B256>,
}

pub trait TokenMetadataProvider {
    fn token_metadata<'a>(
        &'a self,
        lookup: &'a TokenMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<ERC20TokenMetadata>>> + 'a>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UniswapV2PoolMetadataLookup {
    pub tracked_token_address: Option<Address>,
    pub pool_address: Address,
    pub block_number: u64,
    pub transaction_hash: B256,
    pub tx_index: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UniswapV2PoolMetadata {
    pub protocol: KnownV2Protocol,
    pub pool_address: String,
    pub token0: String,
    pub token1: String,
    pub token0_decimals: u8,
    pub token1_decimals: u8,
}

impl UniswapV2PoolMetadata {
    pub fn new(
        pool_address: impl Into<String>,
        token0: impl Into<String>,
        token1: impl Into<String>,
        token0_decimals: u8,
        token1_decimals: u8,
    ) -> Self {
        Self::new_with_protocol(
            KnownV2Protocol::UniswapV2,
            pool_address,
            token0,
            token1,
            token0_decimals,
            token1_decimals,
        )
    }

    pub fn new_with_protocol(
        protocol: KnownV2Protocol,
        pool_address: impl Into<String>,
        token0: impl Into<String>,
        token1: impl Into<String>,
        token0_decimals: u8,
        token1_decimals: u8,
    ) -> Self {
        Self {
            protocol,
            pool_address: normalize_address(pool_address.into()),
            token0: normalize_address(token0.into()),
            token1: normalize_address(token1.into()),
            token0_decimals,
            token1_decimals,
        }
    }
}

pub trait UniswapV2PoolMetadataProvider {
    fn uniswap_v2_pool_metadata<'a>(
        &'a self,
        lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolMetadata>>> + 'a>>;
}

pub trait TokenDiscoveryProvider: TokenMetadataProvider + UniswapV2PoolMetadataProvider {}

impl<T> TokenDiscoveryProvider for T where T: TokenMetadataProvider + UniswapV2PoolMetadataProvider {}

pub(crate) fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

pub(crate) fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}
