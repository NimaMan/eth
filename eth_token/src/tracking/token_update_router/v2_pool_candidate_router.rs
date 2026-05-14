use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use alloy_primitives::Address;
use eyre::Result;
use tx_processor::ProcessedTransaction;

use crate::chain_metadata::UniswapV2PoolIdentity;
use crate::chain_metadata::{UniswapV2PoolIdentityProvider, UniswapV2PoolMetadataLookup};
use crate::tracking::{address_string, same_address_str, TokenRegistry, TrackedTokenIndex};

use super::pool_metadata_lookup::optional_uniswap_v2_pool_identity;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct V2PoolCandidateCache {
    identities: BTreeMap<String, UniswapV2PoolIdentity>,
    irrelevant_by_generation: BTreeMap<V2PoolCandidateCacheGeneration, BTreeSet<String>>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct V2PoolCandidateCacheGeneration {
    token_index_generation: u64,
    registry_tokens: usize,
}

impl V2PoolCandidateCacheGeneration {
    fn current(registry: &TokenRegistry, token_index: &TrackedTokenIndex) -> Self {
        Self {
            token_index_generation: token_index.membership_generation(),
            registry_tokens: registry.tokens.len(),
        }
    }
}

impl V2PoolCandidateCache {
    fn identity(&self, pool_address: &str) -> Option<&UniswapV2PoolIdentity> {
        self.identities.get(pool_address)
    }

    fn remember_identity(&mut self, identity: UniswapV2PoolIdentity) {
        self.identities
            .insert(identity.pool_address.clone(), identity);
    }

    fn is_irrelevant(
        &self,
        generation: V2PoolCandidateCacheGeneration,
        pool_address: &str,
    ) -> bool {
        self.irrelevant_by_generation
            .get(&generation)
            .is_some_and(|pools| pools.contains(pool_address))
    }

    fn remember_irrelevant(
        &mut self,
        generation: V2PoolCandidateCacheGeneration,
        pool_address: &str,
    ) {
        self.irrelevant_by_generation
            .entry(generation)
            .or_default()
            .insert(pool_address.to_string());
    }

    pub(crate) fn prune_generations_before(&mut self, generation: u64) {
        self.irrelevant_by_generation
            .retain(|cached_generation, _| cached_generation.token_index_generation >= generation);
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct V2PoolCandidateRouteProfile {
    pub pool_event_scan_us: u128,
    pub transfer_route_us: u128,
    pub cache_route_us: u128,
    pub identity_lookup_us: u128,
    pub pool_event_count: usize,
    pub transfer_route_hits: usize,
    pub identity_cache_hits: usize,
    pub irrelevant_cache_hits: usize,
    pub irrelevant_cache_inserts: usize,
    pub identity_lookups: usize,
    pub identity_hits: usize,
    pub identity_skipped_by_transfer: usize,
}

pub(super) async fn insert_v2_pool_event_candidates<P>(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    tx: &ProcessedTransaction,
    pool_identity_provider: &P,
    metadata_timeout: Option<Duration>,
    mut candidate_cache: Option<&mut V2PoolCandidateCache>,
    candidates: &mut BTreeSet<String>,
) -> Result<V2PoolCandidateRouteProfile>
where
    P: UniswapV2PoolIdentityProvider,
{
    let mut profile = V2PoolCandidateRouteProfile::default();

    let started = Instant::now();
    let v2_pool_addresses = v2_pool_event_addresses(tx);
    profile.pool_event_scan_us += elapsed_micros(started);
    profile.pool_event_count += v2_pool_addresses.len();

    for pool_address in v2_pool_addresses {
        let pool_address_string = address_string(&pool_address);
        if token_index
            .resolve_token_address(&pool_address_string)
            .is_some()
        {
            continue;
        }

        let started = Instant::now();
        let routed_from_transfers = insert_transfer_routed_candidates_for_pool(
            registry,
            token_index,
            tx,
            &pool_address_string,
            candidates,
        );
        profile.transfer_route_us += elapsed_micros(started);
        if routed_from_transfers > 0 {
            profile.transfer_route_hits += routed_from_transfers;
            profile.identity_skipped_by_transfer += 1;
            continue;
        }

        let cache_generation = V2PoolCandidateCacheGeneration::current(registry, token_index);
        if let Some(cache) = candidate_cache.as_deref_mut() {
            if cache.is_irrelevant(cache_generation, &pool_address_string) {
                profile.irrelevant_cache_hits += 1;
                continue;
            }
            if let Some(identity) = cache.identity(&pool_address_string).cloned() {
                let started = Instant::now();
                let routed =
                    insert_identity_routed_candidates(registry, token_index, &identity, candidates);
                profile.cache_route_us += elapsed_micros(started);
                profile.identity_cache_hits += 1;
                if routed == 0 {
                    cache.remember_irrelevant(cache_generation, &pool_address_string);
                    profile.irrelevant_cache_inserts += 1;
                }
                continue;
            }
        }

        profile.identity_lookups += 1;
        let started = Instant::now();
        let identity = optional_uniswap_v2_pool_identity(
            pool_identity_provider,
            UniswapV2PoolMetadataLookup {
                tracked_token_address: None,
                pool_address,
                block_number: tx.block_number,
                transaction_hash: tx.hash,
                tx_index: tx.tx_index,
            },
            metadata_timeout,
        )
        .await?;
        profile.identity_lookup_us += elapsed_micros(started);

        let Some(identity) = identity else {
            continue;
        };
        profile.identity_hits += 1;
        if let Some(cache) = candidate_cache.as_deref_mut() {
            cache.remember_identity(identity.clone());
        }
        let routed =
            insert_identity_routed_candidates(registry, token_index, &identity, candidates);
        if routed == 0 {
            if let Some(cache) = candidate_cache.as_deref_mut() {
                cache.remember_irrelevant(cache_generation, &pool_address_string);
                profile.irrelevant_cache_inserts += 1;
            }
        }
    }

    Ok(profile)
}

fn insert_identity_routed_candidates(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    identity: &UniswapV2PoolIdentity,
    candidates: &mut BTreeSet<String>,
) -> usize {
    let mut routed = BTreeSet::new();
    if let Some(token_address) = resolve_candidate_token(registry, token_index, &identity.token0) {
        routed.insert(token_address);
    }
    if let Some(token_address) = resolve_candidate_token(registry, token_index, &identity.token1) {
        routed.insert(token_address);
    }
    let routed_count = routed.len();
    candidates.extend(routed);
    routed_count
}

fn insert_transfer_routed_candidates_for_pool(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    tx: &ProcessedTransaction,
    pool_address: &str,
    candidates: &mut BTreeSet<String>,
) -> usize {
    let mut routed = BTreeSet::new();
    for transfer in &tx.erc20_transfers {
        if same_address_str(transfer.from_address, pool_address)
            || same_address_str(transfer.to_address, pool_address)
        {
            let transfer_token = address_string(&transfer.token_address);
            if let Some(token_address) =
                resolve_candidate_token(registry, token_index, &transfer_token)
            {
                routed.insert(token_address);
            }
        }
    }
    let routed_count = routed.len();
    candidates.extend(routed);
    routed_count
}

fn resolve_candidate_token(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    address: &str,
) -> Option<String> {
    if let Some(token_address) = token_index.resolve_token_address(address) {
        if registry.token(token_address).is_some() {
            return Some(token_address.to_string());
        }
        return None;
    }

    let normalized = crate::tracking::normalize_address(address);
    registry.token(&normalized).is_some().then_some(normalized)
}

pub(super) fn v2_pool_event_addresses(tx: &ProcessedTransaction) -> BTreeSet<Address> {
    let mut addresses = BTreeSet::new();
    for event in &tx.uniswap_v2_syncs {
        addresses.insert(event.pair_address);
    }
    for event in &tx.uniswap_v2_swaps {
        addresses.insert(event.pair_address);
    }
    for event in &tx.uniswap_v2_mints {
        addresses.insert(event.pair_address);
    }
    for event in &tx.uniswap_v2_burns {
        addresses.insert(event.pair_address);
    }
    addresses
}

fn elapsed_micros(started: Instant) -> u128 {
    started.elapsed().as_micros()
}
