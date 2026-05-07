use std::future::Future;
use std::pin::Pin;

use alloy_primitives::B256;
use eyre::Result;
use reth_chain_query::RethQueryProvider;

use crate::erc20::ERC20TokenMetadata;

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
}

impl<'a> RethChainMetadataProvider<'a> {
    pub fn new(provider: &'a RethQueryProvider) -> Self {
        Self { provider }
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
        Box::pin(async move { uniswap_v2_pool_metadata(self.provider, lookup).await })
    }
}

pub struct LiveRethChainMetadataProvider<'a> {
    provider: &'a RethQueryProvider,
}

impl<'a> LiveRethChainMetadataProvider<'a> {
    pub fn new(provider: &'a RethQueryProvider) -> Self {
        Self { provider }
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
        Box::pin(async move { uniswap_v2_pool_metadata(self.provider, lookup).await })
    }
}

async fn token_metadata_with_mode(
    provider: &RethQueryProvider,
    lookup: &TokenMetadataLookup,
    mode: RethMetadataMode,
) -> Result<Option<ERC20TokenMetadata>> {
    let metadata = provider
        .get_token_metadata(
            lookup.token_address,
            Some(mode.token_metadata_block(lookup)),
            mode.pending_tx_hashes(lookup),
        )
        .await?;

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
    lookup: &UniswapV2PoolMetadataLookup,
) -> Result<Option<UniswapV2PoolMetadata>> {
    let (token0, token1) = provider
        .uni_v2_get_tokens(lookup.pool_address, Some(lookup.block_number))
        .await?;

    if let Some(tracked_token_address) = lookup.tracked_token_address {
        if token0 != tracked_token_address && token1 != tracked_token_address {
            return Ok(None);
        }
    }

    let (token0_decimals, token1_decimals) = tokio::try_join!(
        provider.get_token_decimals(token0, Some(lookup.block_number)),
        provider.get_token_decimals(token1, Some(lookup.block_number)),
    )?;

    Ok(Some(UniswapV2PoolMetadata::new(
        address_string(&lookup.pool_address),
        address_string(&token0),
        address_string(&token1),
        token0_decimals,
        token1_decimals,
    )))
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
}
