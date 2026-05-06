use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use alloy_primitives::{Address, B256};
use eyre::Result;
use reth_chain_query::RethQueryProvider;

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
    pub token_address: Address,
    pub pool_address: Address,
    pub block_number: u64,
    pub transaction_hash: B256,
    pub tx_index: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UniswapV2PoolMetadata {
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
        Self {
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

pub trait TokenPipelineMetadataProvider:
    TokenMetadataProvider + UniswapV2PoolMetadataProvider
{
}

impl<T> TokenPipelineMetadataProvider for T where
    T: TokenMetadataProvider + UniswapV2PoolMetadataProvider
{
}

pub struct RethTokenMetadataProvider<'a> {
    provider: &'a RethQueryProvider,
}

impl<'a> RethTokenMetadataProvider<'a> {
    pub fn new(provider: &'a RethQueryProvider) -> Self {
        Self { provider }
    }
}

impl TokenMetadataProvider for RethTokenMetadataProvider<'_> {
    fn token_metadata<'a>(
        &'a self,
        lookup: &'a TokenMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<ERC20TokenMetadata>>> + 'a>> {
        Box::pin(async move {
            let metadata = self
                .provider
                .get_token_metadata(
                    lookup.token_address,
                    Some(lookup.metadata_block_number),
                    Some(lookup.pending_tx_hashes.clone()),
                )
                .await?;

            Ok(metadata.map(|metadata| ERC20TokenMetadata {
                address: address_string(&metadata.address),
                name: metadata.name,
                symbol: metadata.symbol,
                decimals: metadata.decimals,
                total_supply: metadata.total_supply.to_string(),
            }))
        })
    }
}

impl UniswapV2PoolMetadataProvider for RethTokenMetadataProvider<'_> {
    fn uniswap_v2_pool_metadata<'a>(
        &'a self,
        lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolMetadata>>> + 'a>> {
        Box::pin(async move {
            let (token0, token1) = self
                .provider
                .uni_v2_get_tokens(lookup.pool_address, Some(lookup.block_number))
                .await?;

            if token0 != lookup.token_address && token1 != lookup.token_address {
                return Ok(None);
            }

            let (token0_decimals, token1_decimals) = tokio::try_join!(
                self.provider
                    .get_token_decimals(token0, Some(lookup.block_number)),
                self.provider
                    .get_token_decimals(token1, Some(lookup.block_number)),
            )?;

            Ok(Some(UniswapV2PoolMetadata::new(
                address_string(&lookup.pool_address),
                address_string(&token0),
                address_string(&token1),
                token0_decimals,
                token1_decimals,
            )))
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct StaticTokenMetadataProvider {
    metadata: HashMap<String, ERC20TokenMetadata>,
}

impl StaticTokenMetadataProvider {
    pub fn new(metadata: impl IntoIterator<Item = ERC20TokenMetadata>) -> Self {
        Self {
            metadata: metadata
                .into_iter()
                .map(|metadata| (normalize_address(&metadata.address), metadata))
                .collect(),
        }
    }

    pub fn insert(&mut self, metadata: ERC20TokenMetadata) -> Option<ERC20TokenMetadata> {
        self.metadata
            .insert(normalize_address(&metadata.address), metadata)
    }
}

impl TokenMetadataProvider for StaticTokenMetadataProvider {
    fn token_metadata<'a>(
        &'a self,
        lookup: &'a TokenMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<ERC20TokenMetadata>>> + 'a>> {
        Box::pin(async move {
            Ok(self
                .metadata
                .get(&address_string(&lookup.token_address))
                .cloned())
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct StaticUniswapV2PoolMetadataProvider {
    metadata: HashMap<String, UniswapV2PoolMetadata>,
}

impl StaticUniswapV2PoolMetadataProvider {
    pub fn new(metadata: impl IntoIterator<Item = UniswapV2PoolMetadata>) -> Self {
        Self {
            metadata: metadata
                .into_iter()
                .map(|metadata| (normalize_address(&metadata.pool_address), metadata))
                .collect(),
        }
    }

    pub fn insert(&mut self, metadata: UniswapV2PoolMetadata) -> Option<UniswapV2PoolMetadata> {
        self.metadata
            .insert(normalize_address(&metadata.pool_address), metadata)
    }
}

impl UniswapV2PoolMetadataProvider for StaticUniswapV2PoolMetadataProvider {
    fn uniswap_v2_pool_metadata<'a>(
        &'a self,
        lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolMetadata>>> + 'a>> {
        Box::pin(async move {
            Ok(self
                .metadata
                .get(&address_string(&lookup.pool_address))
                .cloned())
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct NoopUniswapV2PoolMetadataProvider;

impl UniswapV2PoolMetadataProvider for NoopUniswapV2PoolMetadataProvider {
    fn uniswap_v2_pool_metadata<'a>(
        &'a self,
        _lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolMetadata>>> + 'a>> {
        Box::pin(async { Ok(None) })
    }
}

fn address_string(address: &Address) -> String {
    format!("{address:#x}")
}

fn normalize_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}
