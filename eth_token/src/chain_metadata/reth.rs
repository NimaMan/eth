use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use alloy_primitives::B256;
use eyre::Result;
use reth_chain_query::RethQueryProvider;

use crate::erc20::ERC20TokenMetadata;

use super::cache::RethChainMetadataCache;
use super::types::{
    address_string, TokenMetadataLookup, TokenMetadataProvider, UniswapV2PoolMetadata,
    UniswapV2PoolMetadataLookup, UniswapV2PoolMetadataProvider,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RethMetadataMode {
    Regular,
    Live,
}

impl RethMetadataMode {
    fn token_metadata_block(self, lookup: &TokenMetadataLookup) -> u64 {
        match self {
            // Regular indexing reads the post-block state for same-block
            // deployments, avoiding live Redis pending replay entirely.
            Self::Regular => lookup.block_number,
            Self::Live => lookup.metadata_block_number,
        }
    }

    fn pending_tx_hashes(self, lookup: &TokenMetadataLookup) -> Option<Vec<B256>> {
        match self {
            Self::Regular => None,
            Self::Live => Some(lookup.pending_tx_hashes.clone()),
        }
    }
}

pub struct RethChainMetadataProvider<'a> {
    provider: &'a RethQueryProvider,
    cache: Arc<RethChainMetadataCache>,
}

impl<'a> RethChainMetadataProvider<'a> {
    pub fn new(provider: &'a RethQueryProvider) -> Self {
        Self {
            provider,
            cache: Arc::new(RethChainMetadataCache::default()),
        }
    }
}

impl TokenMetadataProvider for RethChainMetadataProvider<'_> {
    fn token_metadata<'a>(
        &'a self,
        lookup: &'a TokenMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<ERC20TokenMetadata>>> + 'a>> {
        Box::pin(async move {
            token_metadata_with_mode(self.provider, lookup, RethMetadataMode::Regular).await
        })
    }
}

impl UniswapV2PoolMetadataProvider for RethChainMetadataProvider<'_> {
    fn uniswap_v2_pool_metadata<'a>(
        &'a self,
        lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolMetadata>>> + 'a>> {
        Box::pin(async move { uniswap_v2_pool_metadata(self.provider, &self.cache, lookup).await })
    }
}

pub struct LiveRethChainMetadataProvider<'a> {
    provider: &'a RethQueryProvider,
    cache: Arc<RethChainMetadataCache>,
}

impl<'a> LiveRethChainMetadataProvider<'a> {
    pub fn new(provider: &'a RethQueryProvider) -> Self {
        Self {
            provider,
            cache: Arc::new(RethChainMetadataCache::default()),
        }
    }
}

impl TokenMetadataProvider for LiveRethChainMetadataProvider<'_> {
    fn token_metadata<'a>(
        &'a self,
        lookup: &'a TokenMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<ERC20TokenMetadata>>> + 'a>> {
        Box::pin(async move {
            token_metadata_with_mode(self.provider, lookup, RethMetadataMode::Live).await
        })
    }
}

impl UniswapV2PoolMetadataProvider for LiveRethChainMetadataProvider<'_> {
    fn uniswap_v2_pool_metadata<'a>(
        &'a self,
        lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolMetadata>>> + 'a>> {
        Box::pin(async move { uniswap_v2_pool_metadata(self.provider, &self.cache, lookup).await })
    }
}

async fn token_metadata_with_mode(
    provider: &RethQueryProvider,
    lookup: &TokenMetadataLookup,
    mode: RethMetadataMode,
) -> Result<Option<ERC20TokenMetadata>> {
    let metadata = match provider
        .get_token_metadata(
            lookup.token_address,
            Some(mode.token_metadata_block(lookup)),
            mode.pending_tx_hashes(lookup),
        )
        .await
    {
        Ok(metadata) => metadata,
        Err(error) if is_optional_token_metadata_read_error(&error.to_string()) => return Ok(None),
        Err(error) => return Err(error),
    };

    Ok(metadata.map(|metadata| ERC20TokenMetadata {
        address: address_string(&metadata.address),
        name: metadata.name,
        symbol: metadata.symbol,
        decimals: metadata.decimals,
        total_supply: metadata.total_supply.to_string(),
    }))
}

async fn uniswap_v2_pool_metadata(
    provider: &RethQueryProvider,
    cache: &RethChainMetadataCache,
    lookup: &UniswapV2PoolMetadataLookup,
) -> Result<Option<UniswapV2PoolMetadata>> {
    if let Some(metadata) = cache.v2_pool_metadata(lookup.pool_address) {
        return Ok(filter_uniswap_v2_pool_metadata(
            metadata,
            lookup.tracked_token_address,
        ));
    }

    let (token0, token1) = provider
        .uni_v2_get_tokens(lookup.pool_address, Some(lookup.block_number))
        .await?;

    if is_native_eth_sentinel(&token0) || is_native_eth_sentinel(&token1) {
        cache.remember_v2_pool_metadata(lookup.pool_address, None);
        return Ok(None);
    }

    let (token0_decimals, token1_decimals) = tokio::try_join!(
        cached_token_decimals(provider, cache, token0, lookup.block_number),
        cached_token_decimals(provider, cache, token1, lookup.block_number),
    )?;

    let metadata = UniswapV2PoolMetadata::new(
        address_string(&lookup.pool_address),
        address_string(&token0),
        address_string(&token1),
        token0_decimals,
        token1_decimals,
    );
    cache.remember_v2_pool_metadata(lookup.pool_address, Some(metadata.clone()));

    Ok(filter_uniswap_v2_pool_metadata(
        Some(metadata),
        lookup.tracked_token_address,
    ))
}

async fn cached_token_decimals(
    provider: &RethQueryProvider,
    cache: &RethChainMetadataCache,
    token_address: alloy_primitives::Address,
    block_number: u64,
) -> Result<u8> {
    if let Some(decimals) = cache.token_decimals(token_address) {
        return Ok(decimals);
    }
    let decimals = provider
        .get_token_decimals(token_address, Some(block_number))
        .await?;
    cache.remember_token_decimals(token_address, decimals);
    Ok(decimals)
}

fn filter_uniswap_v2_pool_metadata(
    metadata: Option<UniswapV2PoolMetadata>,
    tracked_token_address: Option<alloy_primitives::Address>,
) -> Option<UniswapV2PoolMetadata> {
    let Some(tracked_token_address) = tracked_token_address else {
        return metadata;
    };
    metadata.filter(|metadata| {
        metadata.token0 == address_string(&tracked_token_address)
            || metadata.token1 == address_string(&tracked_token_address)
    })
}

fn is_native_eth_sentinel(address: &alloy_primitives::Address) -> bool {
    address_string(address).eq_ignore_ascii_case("0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")
}

fn is_optional_token_metadata_read_error(message: &str) -> bool {
    // Token metadata is optional for discovery. Contracts that do not fully
    // implement ERC-20 metadata should not make block application fail.
    message.contains("Failed to get token name")
        || message.contains("Failed to get token symbol")
        || message.contains("Failed to get token decimals")
        || message.contains("Token decimals call")
        || message.contains("missing live block header")
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{address, b256};

    use super::*;

    fn lookup() -> TokenMetadataLookup {
        TokenMetadataLookup {
            token_address: address!("1111111111111111111111111111111111111111"),
            block_number: 100,
            block_timestamp: 1_700,
            metadata_block_number: 99,
            transaction_hash: b256!(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            ),
            tx_index: 7,
            creator_address: address!("2222222222222222222222222222222222222222"),
            creator_nonce: 3,
            pending_tx_hashes: vec![b256!(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            )],
        }
    }

    #[test]
    fn regular_mode_uses_post_block_state_without_pending_replay() {
        let lookup = lookup();

        assert_eq!(
            RethMetadataMode::Regular.token_metadata_block(&lookup),
            lookup.block_number
        );
        assert_eq!(RethMetadataMode::Regular.pending_tx_hashes(&lookup), None);
    }

    #[test]
    fn live_mode_uses_parent_state_with_pending_replay() {
        let lookup = lookup();

        assert_eq!(
            RethMetadataMode::Live.token_metadata_block(&lookup),
            lookup.metadata_block_number
        );
        assert_eq!(
            RethMetadataMode::Live.pending_tx_hashes(&lookup),
            Some(lookup.pending_tx_hashes.clone())
        );
    }

    #[test]
    fn native_eth_sentinel_is_not_treated_as_v2_erc20_metadata() {
        assert!(is_native_eth_sentinel(&address!(
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        )));
    }

    #[test]
    fn optional_token_metadata_read_errors_are_not_token_discovery_failures() {
        assert!(is_optional_token_metadata_read_error(
            "Failed to get token decimals for 0x1111111111111111111111111111111111111111"
        ));
        assert!(is_optional_token_metadata_read_error(
            "Token decimals call for 0x1111111111111111111111111111111111111111 returned 0 bytes (expected >= 32)"
        ));
        assert!(is_optional_token_metadata_read_error(
            "missing live block header for block 123"
        ));
        assert!(!is_optional_token_metadata_read_error(
            "transaction validation error: lack of funds"
        ));
    }
}
