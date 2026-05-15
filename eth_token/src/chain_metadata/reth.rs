use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use alloy_primitives::{Address, Bytes, B256, U256};
use eyre::{eyre, Result};
use reth_chain_query::common_addresses::KnownV2Protocol;
use reth_chain_query::dex::{
    compute_fraxswap_v2_pool, compute_pancakeswap_v2_pool, compute_shibaswap_v2_pool,
    compute_sushiswap_pool, compute_uniswap_v2_pool,
};
use reth_chain_query::RethQueryProvider;
use tx_processor::{BlockStateSession, UnsignedTxChainSimulation};

use crate::erc20::ERC20TokenMetadata;

use super::cache::RethChainMetadataCache;
use super::types::{
    address_string, TokenMetadataLookup, TokenMetadataProvider, UniswapV2PoolIdentity,
    UniswapV2PoolIdentityProvider, UniswapV2PoolMetadata, UniswapV2PoolMetadataLookup,
    UniswapV2PoolMetadataProvider,
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
            // Live token tracking is co-located with live block processing. The
            // authoritative live view is the current post-block session; if we
            // fall back to local Reth, use the same post-block state instead of
            // the removed parent-state-plus-pending-replay path.
            Self::Live => lookup.block_number,
        }
    }

    fn pending_tx_hashes(self, _lookup: &TokenMetadataLookup) -> Option<Vec<B256>> {
        match self {
            Self::Regular => None,
            Self::Live => None,
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

impl UniswapV2PoolIdentityProvider for RethChainMetadataProvider<'_> {
    fn uniswap_v2_pool_identity<'a>(
        &'a self,
        lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolIdentity>>> + 'a>> {
        Box::pin(async move { uniswap_v2_pool_identity(self.provider, &self.cache, lookup).await })
    }
}

pub struct LiveRethChainMetadataProvider<'a> {
    provider: &'a RethQueryProvider,
    cache: Arc<RethChainMetadataCache>,
    direct_live_block_sessions: Option<&'a Mutex<BTreeMap<u64, BlockStateSession>>>,
}

impl<'a> LiveRethChainMetadataProvider<'a> {
    pub fn new(provider: &'a RethQueryProvider) -> Self {
        Self {
            provider,
            cache: Arc::new(RethChainMetadataCache::default()),
            direct_live_block_sessions: None,
        }
    }

    pub fn with_direct_live_block_sessions(
        provider: &'a RethQueryProvider,
        direct_live_block_sessions: &'a Mutex<BTreeMap<u64, BlockStateSession>>,
    ) -> Self {
        Self {
            provider,
            cache: Arc::new(RethChainMetadataCache::default()),
            direct_live_block_sessions: Some(direct_live_block_sessions),
        }
    }

    fn direct_live_chain(&self, block_number: u64) -> Result<Option<UnsignedTxChainSimulation>> {
        let Some(sessions) = self.direct_live_block_sessions else {
            return Ok(None);
        };

        let session = sessions
            .lock()
            .map_err(|error| eyre!("direct live block session lock poisoned: {error}"))?
            .get(&block_number)
            .cloned();
        Ok(session.map(|session| session.simulation_chain()))
    }
}

impl TokenMetadataProvider for LiveRethChainMetadataProvider<'_> {
    fn token_metadata<'a>(
        &'a self,
        lookup: &'a TokenMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<ERC20TokenMetadata>>> + 'a>> {
        Box::pin(async move {
            if let Some(mut chain) = self.direct_live_chain(lookup.block_number)? {
                return token_metadata_from_direct_live_chain(&mut chain, lookup);
            }

            token_metadata_with_mode(self.provider, lookup, RethMetadataMode::Live).await
        })
    }
}

impl UniswapV2PoolMetadataProvider for LiveRethChainMetadataProvider<'_> {
    fn uniswap_v2_pool_metadata<'a>(
        &'a self,
        lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolMetadata>>> + 'a>> {
        Box::pin(async move {
            if let Some(mut chain) = self.direct_live_chain(lookup.block_number)? {
                return uniswap_v2_pool_metadata_from_direct_live_chain(
                    &mut chain,
                    &self.cache,
                    lookup,
                );
            }

            uniswap_v2_pool_metadata(self.provider, &self.cache, lookup).await
        })
    }
}

impl UniswapV2PoolIdentityProvider for LiveRethChainMetadataProvider<'_> {
    fn uniswap_v2_pool_identity<'a>(
        &'a self,
        lookup: &'a UniswapV2PoolMetadataLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolIdentity>>> + 'a>> {
        Box::pin(async move {
            if let Some(mut chain) = self.direct_live_chain(lookup.block_number)? {
                return uniswap_v2_pool_identity_from_direct_live_chain(
                    &mut chain,
                    &self.cache,
                    lookup,
                );
            }

            uniswap_v2_pool_identity(self.provider, &self.cache, lookup).await
        })
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

fn token_metadata_from_direct_live_chain(
    chain: &mut UnsignedTxChainSimulation,
    lookup: &TokenMetadataLookup,
) -> Result<Option<ERC20TokenMetadata>> {
    let address = lookup.token_address;
    if !chain.account_has_code(address)? {
        return Ok(None);
    }

    let Some(total_supply) = direct_uint256_view(chain, address, SELECTOR_TOTAL_SUPPLY)? else {
        return Ok(None);
    };
    if direct_uint256_view(
        chain,
        address,
        calldata_with_address(SELECTOR_BALANCE_OF, Address::ZERO),
    )?
    .is_none()
    {
        return Ok(None);
    }
    if direct_uint256_view(
        chain,
        address,
        calldata_with_two_addresses(SELECTOR_ALLOWANCE, Address::ZERO, Address::ZERO),
    )?
    .is_none()
    {
        return Ok(None);
    }

    let Some(decimals) = direct_u8_view(chain, address, SELECTOR_DECIMALS)? else {
        return Ok(None);
    };
    let Some(name) = direct_string_view(chain, address, SELECTOR_NAME)? else {
        return Ok(None);
    };
    let Some(symbol) = direct_string_view(chain, address, SELECTOR_SYMBOL)? else {
        return Ok(None);
    };

    Ok(Some(ERC20TokenMetadata {
        address: address_string(&address),
        name,
        symbol,
        decimals,
        total_supply: total_supply.to_string(),
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

    let Some(identity) = uniswap_v2_pool_identity(provider, cache, lookup).await? else {
        return Ok(None);
    };
    let token0: Address = identity.token0.parse()?;
    let token1: Address = identity.token1.parse()?;

    let (token0_decimals, token1_decimals) = tokio::try_join!(
        cached_token_decimals(provider, cache, token0, lookup.block_number),
        cached_token_decimals(provider, cache, token1, lookup.block_number),
    )?;

    let metadata = UniswapV2PoolMetadata::new_with_protocol(
        identity.protocol,
        address_string(&lookup.pool_address),
        identity.token0,
        identity.token1,
        token0_decimals,
        token1_decimals,
    );
    cache.remember_v2_pool_metadata(lookup.pool_address, Some(metadata.clone()));

    Ok(filter_uniswap_v2_pool_metadata(
        Some(metadata),
        lookup.tracked_token_address,
    ))
}

fn uniswap_v2_pool_metadata_from_direct_live_chain(
    chain: &mut UnsignedTxChainSimulation,
    cache: &RethChainMetadataCache,
    lookup: &UniswapV2PoolMetadataLookup,
) -> Result<Option<UniswapV2PoolMetadata>> {
    if let Some(metadata) = cache.v2_pool_metadata(lookup.pool_address) {
        return Ok(filter_uniswap_v2_pool_metadata(
            metadata,
            lookup.tracked_token_address,
        ));
    }

    let Some(identity) = uniswap_v2_pool_identity_from_direct_live_chain(chain, cache, lookup)?
    else {
        return Ok(None);
    };
    let token0: Address = identity.token0.parse()?;
    let token1: Address = identity.token1.parse()?;

    let Some(token0_decimals) = direct_cached_token_decimals(chain, cache, token0)? else {
        return Ok(None);
    };
    let Some(token1_decimals) = direct_cached_token_decimals(chain, cache, token1)? else {
        return Ok(None);
    };

    let metadata = UniswapV2PoolMetadata::new_with_protocol(
        identity.protocol,
        address_string(&lookup.pool_address),
        identity.token0,
        identity.token1,
        token0_decimals,
        token1_decimals,
    );
    cache.remember_v2_pool_metadata(lookup.pool_address, Some(metadata.clone()));

    Ok(filter_uniswap_v2_pool_metadata(
        Some(metadata),
        lookup.tracked_token_address,
    ))
}

async fn uniswap_v2_pool_identity(
    provider: &RethQueryProvider,
    cache: &RethChainMetadataCache,
    lookup: &UniswapV2PoolMetadataLookup,
) -> Result<Option<UniswapV2PoolIdentity>> {
    if let Some(metadata) = cache.v2_pool_metadata(lookup.pool_address) {
        return Ok(filter_uniswap_v2_pool_identity(
            metadata.as_ref().map(UniswapV2PoolIdentity::from),
            lookup.tracked_token_address,
        ));
    }

    if let Some(identity) = cache.v2_pool_identity(lookup.pool_address) {
        return Ok(filter_uniswap_v2_pool_identity(
            identity,
            lookup.tracked_token_address,
        ));
    }

    let (token0, token1) = provider
        .uni_v2_get_tokens(lookup.pool_address, Some(lookup.block_number))
        .await?;

    if is_native_eth_sentinel(&token0) || is_native_eth_sentinel(&token1) {
        cache.remember_v2_pool_identity(lookup.pool_address, None);
        return Ok(None);
    }

    let factory = provider
        .uni_v2_get_factory(lookup.pool_address, Some(lookup.block_number))
        .await?;
    let Some(protocol) = KnownV2Protocol::from_factory(factory) else {
        tracing::debug!(
            pool_address = %lookup.pool_address,
            factory = %factory,
            token0 = %token0,
            token1 = %token1,
            block_number = lookup.block_number,
            tx_index = lookup.tx_index,
            tx_hash = %lookup.transaction_hash,
            "skipped unknown v2-style pool factory"
        );
        cache.remember_v2_pool_identity(lookup.pool_address, None);
        return Ok(None);
    };

    let resolved_pair = compute_known_v2_protocol_pool_address(protocol, token0, token1);
    if !is_known_v2_protocol_pool(lookup.pool_address, resolved_pair) {
        tracing::debug!(
            pool_address = %lookup.pool_address,
            protocol = protocol.label(),
            factory = %factory,
            token0 = %token0,
            token1 = %token1,
            resolved_pair = %resolved_pair,
            block_number = lookup.block_number,
            tx_index = lookup.tx_index,
            tx_hash = %lookup.transaction_hash,
            "skipped mismatched known v2 pool"
        );
        cache.remember_v2_pool_identity(lookup.pool_address, None);
        return Ok(None);
    }

    let identity = UniswapV2PoolIdentity::new_with_protocol(
        protocol,
        address_string(&lookup.pool_address),
        address_string(&token0),
        address_string(&token1),
    );
    cache.remember_v2_pool_identity(lookup.pool_address, Some(identity.clone()));

    Ok(filter_uniswap_v2_pool_identity(
        Some(identity),
        lookup.tracked_token_address,
    ))
}

fn uniswap_v2_pool_identity_from_direct_live_chain(
    chain: &mut UnsignedTxChainSimulation,
    cache: &RethChainMetadataCache,
    lookup: &UniswapV2PoolMetadataLookup,
) -> Result<Option<UniswapV2PoolIdentity>> {
    if let Some(metadata) = cache.v2_pool_metadata(lookup.pool_address) {
        return Ok(filter_uniswap_v2_pool_identity(
            metadata.as_ref().map(UniswapV2PoolIdentity::from),
            lookup.tracked_token_address,
        ));
    }

    if let Some(identity) = cache.v2_pool_identity(lookup.pool_address) {
        return Ok(filter_uniswap_v2_pool_identity(
            identity,
            lookup.tracked_token_address,
        ));
    }

    let Some(token0) = direct_address_view(chain, lookup.pool_address, SELECTOR_TOKEN0)? else {
        cache.remember_v2_pool_identity(lookup.pool_address, None);
        return Ok(None);
    };
    let Some(token1) = direct_address_view(chain, lookup.pool_address, SELECTOR_TOKEN1)? else {
        cache.remember_v2_pool_identity(lookup.pool_address, None);
        return Ok(None);
    };

    if is_native_eth_sentinel(&token0) || is_native_eth_sentinel(&token1) {
        cache.remember_v2_pool_identity(lookup.pool_address, None);
        return Ok(None);
    }

    let Some(factory) = direct_address_view(chain, lookup.pool_address, SELECTOR_FACTORY)? else {
        cache.remember_v2_pool_identity(lookup.pool_address, None);
        return Ok(None);
    };
    let Some(protocol) = KnownV2Protocol::from_factory(factory) else {
        tracing::debug!(
            pool_address = %lookup.pool_address,
            factory = %factory,
            token0 = %token0,
            token1 = %token1,
            block_number = lookup.block_number,
            tx_index = lookup.tx_index,
            tx_hash = %lookup.transaction_hash,
            source = "direct_live_block_session",
            "skipped unknown v2-style pool factory"
        );
        cache.remember_v2_pool_identity(lookup.pool_address, None);
        return Ok(None);
    };

    let resolved_pair = compute_known_v2_protocol_pool_address(protocol, token0, token1);
    if !is_known_v2_protocol_pool(lookup.pool_address, resolved_pair) {
        tracing::debug!(
            pool_address = %lookup.pool_address,
            protocol = protocol.label(),
            factory = %factory,
            token0 = %token0,
            token1 = %token1,
            resolved_pair = %resolved_pair,
            block_number = lookup.block_number,
            tx_index = lookup.tx_index,
            tx_hash = %lookup.transaction_hash,
            source = "direct_live_block_session",
            "skipped mismatched known v2 pool"
        );
        cache.remember_v2_pool_identity(lookup.pool_address, None);
        return Ok(None);
    }

    let identity = UniswapV2PoolIdentity::new_with_protocol(
        protocol,
        address_string(&lookup.pool_address),
        address_string(&token0),
        address_string(&token1),
    );
    cache.remember_v2_pool_identity(lookup.pool_address, Some(identity.clone()));

    Ok(filter_uniswap_v2_pool_identity(
        Some(identity),
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

fn direct_cached_token_decimals(
    chain: &mut UnsignedTxChainSimulation,
    cache: &RethChainMetadataCache,
    token_address: Address,
) -> Result<Option<u8>> {
    if let Some(decimals) = cache.token_decimals(token_address) {
        return Ok(Some(decimals));
    }

    let Some(decimals) = direct_u8_view(chain, token_address, SELECTOR_DECIMALS)? else {
        return Ok(None);
    };
    cache.remember_token_decimals(token_address, decimals);
    Ok(Some(decimals))
}

fn filter_uniswap_v2_pool_identity(
    identity: Option<UniswapV2PoolIdentity>,
    tracked_token_address: Option<alloy_primitives::Address>,
) -> Option<UniswapV2PoolIdentity> {
    let Some(tracked_token_address) = tracked_token_address else {
        return identity;
    };
    identity.filter(|identity| {
        identity.token0 == address_string(&tracked_token_address)
            || identity.token1 == address_string(&tracked_token_address)
    })
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

fn is_known_v2_protocol_pool(pool_address: Address, resolved_pair: Address) -> bool {
    !resolved_pair.is_zero() && resolved_pair == pool_address
}

fn compute_known_v2_protocol_pool_address(
    protocol: KnownV2Protocol,
    token0: Address,
    token1: Address,
) -> Address {
    match protocol {
        KnownV2Protocol::UniswapV2 => compute_uniswap_v2_pool(token0, token1),
        KnownV2Protocol::SushiSwapV2 => compute_sushiswap_pool(token0, token1),
        KnownV2Protocol::PancakeSwapV2 => compute_pancakeswap_v2_pool(token0, token1),
        KnownV2Protocol::ShibaSwapV2 => compute_shibaswap_v2_pool(token0, token1),
        KnownV2Protocol::FraxswapV2 => compute_fraxswap_v2_pool(token0, token1),
    }
}

const SELECTOR_TOTAL_SUPPLY: [u8; 4] = [0x18, 0x16, 0x0d, 0xdd];
const SELECTOR_BALANCE_OF: [u8; 4] = [0x70, 0xa0, 0x82, 0x31];
const SELECTOR_ALLOWANCE: [u8; 4] = [0xdd, 0x62, 0xed, 0x3e];
const SELECTOR_DECIMALS: [u8; 4] = [0x31, 0x3c, 0xe5, 0x67];
const SELECTOR_NAME: [u8; 4] = [0x06, 0xfd, 0xde, 0x03];
const SELECTOR_SYMBOL: [u8; 4] = [0x95, 0xd8, 0x9b, 0x41];
const SELECTOR_TOKEN0: [u8; 4] = [0x0d, 0xfe, 0x16, 0x81];
const SELECTOR_TOKEN1: [u8; 4] = [0xd2, 0x12, 0x20, 0xa7];
const SELECTOR_FACTORY: [u8; 4] = [0xc4, 0x5a, 0x01, 0x55];

fn direct_uint256_view(
    chain: &mut UnsignedTxChainSimulation,
    contract: Address,
    data: impl Into<Bytes>,
) -> Result<Option<U256>> {
    let Some(output) = direct_view_output(chain, contract, data.into())? else {
        return Ok(None);
    };
    if output.len() < 32 {
        return Ok(None);
    }
    Ok(Some(U256::from_be_slice(&output[..32])))
}

fn direct_u8_view(
    chain: &mut UnsignedTxChainSimulation,
    contract: Address,
    selector: [u8; 4],
) -> Result<Option<u8>> {
    let Some(output) = direct_view_output(chain, contract, Bytes::copy_from_slice(&selector))?
    else {
        return Ok(None);
    };
    if output.len() < 32 {
        return Ok(None);
    }
    Ok(Some(output[31]))
}

fn direct_string_view(
    chain: &mut UnsignedTxChainSimulation,
    contract: Address,
    selector: [u8; 4],
) -> Result<Option<String>> {
    let Some(output) = direct_view_output(chain, contract, Bytes::copy_from_slice(&selector))?
    else {
        return Ok(None);
    };
    Ok(decode_metadata_string(&output))
}

fn direct_address_view(
    chain: &mut UnsignedTxChainSimulation,
    contract: Address,
    selector: [u8; 4],
) -> Result<Option<Address>> {
    let Some(output) = direct_view_output(chain, contract, Bytes::copy_from_slice(&selector))?
    else {
        return Ok(None);
    };
    if output.len() < 32 {
        return Ok(None);
    }
    Ok(Some(Address::from_slice(&output[12..32])))
}

fn direct_view_output(
    chain: &mut UnsignedTxChainSimulation,
    contract: Address,
    data: Bytes,
) -> Result<Option<Bytes>> {
    match chain.simulate_view_call(contract, data) {
        Ok(result) if result.success => Ok(Some(result.output)),
        Ok(_) => Ok(None),
        Err(error) => {
            tracing::debug!(
                contract = %contract,
                error = %error,
                "direct live metadata view call failed"
            );
            Ok(None)
        }
    }
}

fn calldata_with_address(selector: [u8; 4], address: Address) -> Bytes {
    let mut payload = Vec::with_capacity(36);
    payload.extend_from_slice(&selector);
    payload.extend_from_slice(&[0u8; 12]);
    payload.extend_from_slice(address.as_slice());
    Bytes::from(payload)
}

fn calldata_with_two_addresses(selector: [u8; 4], first: Address, second: Address) -> Bytes {
    let mut payload = Vec::with_capacity(68);
    payload.extend_from_slice(&selector);
    payload.extend_from_slice(&[0u8; 12]);
    payload.extend_from_slice(first.as_slice());
    payload.extend_from_slice(&[0u8; 12]);
    payload.extend_from_slice(second.as_slice());
    Bytes::from(payload)
}

fn decode_metadata_string(output: &[u8]) -> Option<String> {
    if output.is_empty() {
        return None;
    }

    if output.len() >= 64 {
        let offset = U256::from_be_slice(&output[..32]).to::<usize>();
        if offset
            .checked_add(32)
            .is_some_and(|end| end <= output.len())
        {
            let length = U256::from_be_slice(&output[offset..offset + 32]).to::<usize>();
            let start = offset + 32;
            if start
                .checked_add(length)
                .is_some_and(|end| end <= output.len())
            {
                return String::from_utf8(output[start..start + length].to_vec()).ok();
            }
        }
    }

    if output.len() == 32 {
        let end = output
            .iter()
            .rposition(|byte| *byte != 0)
            .map(|index| index + 1)
            .unwrap_or(0);
        let trimmed = &output[..end];
        if !trimmed.is_empty() {
            return String::from_utf8(trimmed.to_vec()).ok();
        }
    }

    String::from_utf8(output.to_vec()).ok()
}

fn is_optional_token_metadata_read_error(message: &str) -> bool {
    // Token metadata is optional for discovery. Contracts that do not fully
    // implement ERC-20 metadata should not make block application fail.
    message.contains("Failed to get token name")
        || message.contains("Failed to get token symbol")
        || message.contains("Failed to get token decimals")
        || message.contains("Token decimals call")
        || message.contains("missing live block header")
        || message.contains("live chain cache not configured")
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
    fn live_mode_uses_post_block_state_without_pending_replay() {
        let lookup = lookup();

        assert_eq!(
            RethMetadataMode::Live.token_metadata_block(&lookup),
            lookup.block_number
        );
        assert_eq!(RethMetadataMode::Live.pending_tx_hashes(&lookup), None);
    }

    #[test]
    fn native_eth_sentinel_is_not_treated_as_v2_erc20_metadata() {
        assert!(is_native_eth_sentinel(&address!(
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        )));
    }

    #[test]
    fn known_v2_protocol_pool_requires_factory_match() {
        let pool = address!("3333333333333333333333333333333333333333");

        assert!(is_known_v2_protocol_pool(pool, pool));
        assert!(!is_known_v2_protocol_pool(pool, Address::ZERO));
        assert!(!is_known_v2_protocol_pool(
            pool,
            address!("4444444444444444444444444444444444444444")
        ));
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
        assert!(is_optional_token_metadata_read_error(
            "live chain cache not configured"
        ));
        assert!(!is_optional_token_metadata_read_error(
            "transaction validation error: lack of funds"
        ));
    }
}
