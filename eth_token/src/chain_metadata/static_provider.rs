use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use eyre::Result;

use crate::erc20::ERC20TokenMetadata;

use super::types::{
    address_string, normalize_address, TokenMetadataLookup, TokenMetadataProvider,
    UniswapV2PoolIdentity, UniswapV2PoolIdentityProvider, UniswapV2PoolMetadata,
    UniswapV2PoolMetadataLookup, UniswapV2PoolMetadataProvider,
};

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

impl UniswapV2PoolIdentityProvider for StaticUniswapV2PoolMetadataProvider {
    fn uniswap_v2_pool_identity<'a>(
        &'a self,
        lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolIdentity>>> + 'a>> {
        Box::pin(async move {
            Ok(self
                .metadata
                .get(&address_string(&lookup.pool_address))
                .map(UniswapV2PoolIdentity::from)
                .filter(|identity| matches_tracked_token(identity, lookup)))
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

impl UniswapV2PoolIdentityProvider for NoopUniswapV2PoolMetadataProvider {
    fn uniswap_v2_pool_identity<'a>(
        &'a self,
        _lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolIdentity>>> + 'a>> {
        Box::pin(async { Ok(None) })
    }
}

fn matches_tracked_token(
    identity: &UniswapV2PoolIdentity,
    lookup: &UniswapV2PoolMetadataLookup,
) -> bool {
    let Some(tracked_token_address) = lookup.tracked_token_address else {
        return true;
    };
    let tracked = address_string(&tracked_token_address);
    identity.token0 == tracked || identity.token1 == tracked
}
