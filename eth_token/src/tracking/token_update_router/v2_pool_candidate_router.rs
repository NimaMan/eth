use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use alloy_primitives::Address;
use eyre::Result;
use tx_processor::ProcessedTransaction;

use crate::chain_metadata::{UniswapV2PoolIdentityProvider, UniswapV2PoolMetadataLookup};
use crate::tracking::{
    address_string, normalize_address, same_address_str, TokenRegistry, TrackedTokenIndex,
};

use super::pool_metadata_lookup::optional_uniswap_v2_pool_identity;
use super::token_candidates::insert_resolved_token_address_str;

#[derive(Clone, Debug, Default)]
pub(super) struct V2PoolCandidateRouteProfile {
    pub pool_event_scan_us: u128,
    pub transfer_route_us: u128,
    pub identity_lookup_us: u128,
    pub pool_event_count: usize,
    pub transfer_route_hits: usize,
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
        insert_resolved_token_address_str(registry, token_index, candidates, &identity.token0);
        insert_resolved_token_address_str(registry, token_index, candidates, &identity.token1);
    }

    Ok(profile)
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
            if let Some(token_address) = token_index.resolve_token_address(&transfer_token) {
                if registry.token(token_address).is_some() {
                    routed.insert(token_address.to_string());
                }
            } else {
                let normalized = normalize_address(&transfer_token);
                if registry.token(&normalized).is_some() {
                    routed.insert(normalized);
                }
            }
        }
    }
    let routed_count = routed.len();
    candidates.extend(routed);
    routed_count
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
