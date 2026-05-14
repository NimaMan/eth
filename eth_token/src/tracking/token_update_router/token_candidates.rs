use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use alloy_primitives::Address;
use eyre::Result;
use tx_processor::ProcessedTransaction;

use crate::chain_metadata::UniswapV2PoolIdentityProvider;
use crate::pools::uniswap::v4_event_display_key;
use crate::tracking::{address_string, normalize_address, TokenRegistry, TrackedTokenIndex};

use super::v2_pool_candidate_router::{insert_v2_pool_event_candidates, v2_pool_event_addresses};

#[derive(Clone, Debug, Default)]
pub(super) struct CandidateTokenAddressProfile {
    pub routing_addresses_us: u128,
    pub resolve_addresses_us: u128,
    pub v4_pool_keys_us: u128,
    pub v3_position_transfer_us: u128,
    pub v4_position_transfer_us: u128,
    pub v4_position_approval_us: u128,
    pub v2_pair_created_us: u128,
    pub v2_pool_event_scan_us: u128,
    pub v2_transfer_route_us: u128,
    pub v2_identity_lookup_us: u128,
    pub finalize_us: u128,
    pub routing_address_count: usize,
    pub v4_pool_key_count: usize,
    pub v2_pool_event_count: usize,
    pub v2_transfer_route_hits: usize,
    pub v2_identity_lookups: usize,
    pub v2_identity_hits: usize,
    pub v2_identity_skipped_by_transfer: usize,
    pub position_scan_tokens: usize,
}

pub(super) fn candidate_token_addresses(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    tx: &ProcessedTransaction,
) -> Vec<String> {
    let mut candidates = BTreeSet::new();
    for address in routing_addresses(tx) {
        insert_resolved_token_address(registry, token_index, &mut candidates, address);
    }
    for pool_key in v4_pool_event_keys(tx) {
        insert_resolved_token_address_str(registry, token_index, &mut candidates, &pool_key);
    }
    insert_v3_position_transfer_candidates(registry, tx, &mut candidates);
    insert_v4_position_transfer_candidates(registry, tx, &mut candidates);
    insert_v4_position_approval_candidates(registry, tx, &mut candidates);
    candidates.into_iter().collect()
}

pub(super) async fn candidate_token_addresses_with_pool_discovery<P>(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    tx: &ProcessedTransaction,
    pool_metadata_provider: &P,
    metadata_timeout: Option<Duration>,
) -> Result<Vec<String>>
where
    P: UniswapV2PoolIdentityProvider,
{
    let (candidates, _) = candidate_token_addresses_with_pool_discovery_and_profile(
        registry,
        token_index,
        tx,
        pool_metadata_provider,
        metadata_timeout,
    )
    .await?;
    Ok(candidates)
}

pub(super) async fn candidate_token_addresses_with_pool_discovery_and_profile<P>(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    tx: &ProcessedTransaction,
    pool_metadata_provider: &P,
    metadata_timeout: Option<Duration>,
) -> Result<(Vec<String>, CandidateTokenAddressProfile)>
where
    P: UniswapV2PoolIdentityProvider,
{
    let mut profile = CandidateTokenAddressProfile::default();
    let mut candidates = BTreeSet::new();

    let started = Instant::now();
    let routing_addresses = routing_addresses(tx);
    profile.routing_addresses_us += elapsed_micros(started);
    profile.routing_address_count += routing_addresses.len();

    let started = Instant::now();
    for address in routing_addresses {
        insert_resolved_token_address(registry, token_index, &mut candidates, address);
    }
    profile.resolve_addresses_us += elapsed_micros(started);

    let started = Instant::now();
    let v4_pool_keys = v4_pool_event_keys(tx);
    profile.v4_pool_key_count += v4_pool_keys.len();
    for pool_key in v4_pool_keys {
        insert_resolved_token_address_str(registry, token_index, &mut candidates, &pool_key);
    }
    profile.v4_pool_keys_us += elapsed_micros(started);

    let started = Instant::now();
    if !tx.erc721_transfers.is_empty() {
        profile.position_scan_tokens += registry.tokens.len();
    }
    insert_v3_position_transfer_candidates(registry, tx, &mut candidates);
    profile.v3_position_transfer_us += elapsed_micros(started);

    let started = Instant::now();
    if !tx.erc721_transfers.is_empty() {
        profile.position_scan_tokens += registry.tokens.len();
    }
    insert_v4_position_transfer_candidates(registry, tx, &mut candidates);
    profile.v4_position_transfer_us += elapsed_micros(started);

    let started = Instant::now();
    if !tx.erc721_approval_events.is_empty() || !tx.approval_for_all_events.is_empty() {
        profile.position_scan_tokens += registry.tokens.len();
    }
    insert_v4_position_approval_candidates(registry, tx, &mut candidates);
    profile.v4_position_approval_us += elapsed_micros(started);

    let started = Instant::now();
    insert_v2_pair_created_candidates(registry, token_index, tx, &mut candidates);
    profile.v2_pair_created_us += elapsed_micros(started);

    if metadata_timeout.is_some() {
        let started = Instant::now();
        let candidates = candidates.into_iter().collect();
        profile.finalize_us += elapsed_micros(started);
        return Ok((candidates, profile));
    }

    let v2_profile = insert_v2_pool_event_candidates(
        registry,
        token_index,
        tx,
        pool_metadata_provider,
        metadata_timeout,
        &mut candidates,
    )
    .await?;
    profile.v2_pool_event_scan_us += v2_profile.pool_event_scan_us;
    profile.v2_transfer_route_us += v2_profile.transfer_route_us;
    profile.v2_identity_lookup_us += v2_profile.identity_lookup_us;
    profile.v2_pool_event_count += v2_profile.pool_event_count;
    profile.v2_transfer_route_hits += v2_profile.transfer_route_hits;
    profile.v2_identity_lookups += v2_profile.identity_lookups;
    profile.v2_identity_hits += v2_profile.identity_hits;
    profile.v2_identity_skipped_by_transfer += v2_profile.identity_skipped_by_transfer;

    let started = Instant::now();
    let candidates = candidates.into_iter().collect();
    profile.finalize_us += elapsed_micros(started);
    Ok((candidates, profile))
}

fn insert_v2_pair_created_candidates(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    tx: &ProcessedTransaction,
    candidates: &mut BTreeSet<String>,
) {
    for event in &tx.uniswap_v2_pair_created_events {
        insert_resolved_token_address(registry, token_index, candidates, event.token0);
        insert_resolved_token_address(registry, token_index, candidates, event.token1);
    }
}

fn insert_v3_position_transfer_candidates(
    registry: &TokenRegistry,
    tx: &ProcessedTransaction,
    candidates: &mut BTreeSet<String>,
) {
    if tx.erc721_transfers.is_empty() {
        return;
    }

    for (token_address, token) in &registry.tokens {
        if token
            .v3_pools
            .values()
            .any(|pool| pool.touches_position_transfer(tx))
        {
            candidates.insert(token_address.clone());
        }
    }
}

fn insert_v4_position_transfer_candidates(
    registry: &TokenRegistry,
    tx: &ProcessedTransaction,
    candidates: &mut BTreeSet<String>,
) {
    if tx.erc721_transfers.is_empty() {
        return;
    }

    for (token_address, token) in &registry.tokens {
        if token
            .v4_pools
            .values()
            .any(|pool| pool.touches_position_transfer(tx))
        {
            candidates.insert(token_address.clone());
        }
    }
}

fn insert_v4_position_approval_candidates(
    registry: &TokenRegistry,
    tx: &ProcessedTransaction,
    candidates: &mut BTreeSet<String>,
) {
    if tx.erc721_approval_events.is_empty() && tx.approval_for_all_events.is_empty() {
        return;
    }

    for (token_address, token) in &registry.tokens {
        if token
            .v4_pools
            .values()
            .any(|pool| pool.touches_position_approval(tx))
        {
            candidates.insert(token_address.clone());
        }
    }
}

fn routing_addresses(tx: &ProcessedTransaction) -> BTreeSet<Address> {
    let mut addresses = BTreeSet::new();

    addresses.extend(tx.unique_addresses.iter().copied());
    addresses.extend(tx.erc20_contracts.iter().copied());
    if let Some(address) = tx.contract_address {
        addresses.insert(address);
    }

    for transfer in &tx.erc20_transfers {
        addresses.insert(transfer.token_address);
    }
    for approval in &tx.erc20_approval_events {
        addresses.insert(approval.token_address);
    }
    for event in &tx.trading_enabled_events {
        addresses.insert(event.token_address);
    }
    for event in &tx.trading_disabled_events {
        addresses.insert(event.token_address);
    }
    for event in &tx.contract_creation_events {
        addresses.insert(event.contract_address);
    }
    for event in &tx.ownership_transferred_events {
        addresses.insert(event.contract_address);
    }
    for event in &tx.ownership_transfer_started_events {
        addresses.insert(event.contract_address);
    }
    for event in &tx.access_control_role_granted_events {
        addresses.insert(event.contract_address);
    }
    for event in &tx.access_control_role_revoked_events {
        addresses.insert(event.contract_address);
    }
    for event in &tx.proxy_admin_changed_events {
        addresses.insert(event.contract_address);
    }

    for event in &tx.uniswap_v2_pair_created_events {
        addresses.insert(event.pair_address);
        addresses.insert(event.token0);
        addresses.insert(event.token1);
        if !event.factory_address.is_zero() {
            addresses.insert(event.factory_address);
        }
    }
    addresses.extend(v2_pool_event_addresses(tx));
    for event in &tx.uniswap_v3_pools {
        if !event.factory_address.is_zero() {
            addresses.insert(event.factory_address);
        }
        addresses.insert(event.pool);
        addresses.insert(event.token0);
        addresses.insert(event.token1);
    }
    addresses.extend(v3_pool_event_addresses(tx));
    for event in &tx.uniswap_v4_initializes {
        addresses.insert(event.pool_manager_address);
        addresses.insert(event.currency0);
        addresses.insert(event.currency1);
        addresses.insert(event.hooks);
    }
    for event in &tx.uniswap_v4_modifies {
        addresses.insert(event.pool_manager_address);
        addresses.insert(event.sender);
    }
    for event in &tx.uniswap_v4_swaps {
        addresses.insert(event.pool_manager_address);
        addresses.insert(event.sender);
    }
    for event in &tx.uniswap_v4_donates {
        addresses.insert(event.pool_manager_address);
        addresses.insert(event.sender);
    }

    addresses
}

fn elapsed_micros(started: Instant) -> u128 {
    started.elapsed().as_micros()
}

pub(super) fn v3_pool_event_addresses(tx: &ProcessedTransaction) -> BTreeSet<Address> {
    let mut addresses = BTreeSet::new();
    for event in &tx.uniswap_v3_initializations {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_swaps {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_mints {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_burns {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_positions {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_increases {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_decreases {
        addresses.insert(event.pool_address);
    }
    addresses
}

pub(super) fn v4_pool_event_keys(tx: &ProcessedTransaction) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for event in &tx.uniswap_v4_initializes {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    for event in &tx.uniswap_v4_modifies {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    for event in &tx.uniswap_v4_swaps {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    for event in &tx.uniswap_v4_donates {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    for event in &tx.uniswap_v4_protocol_fee_updates {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    for event in &tx.uniswap_v4_dynamic_lp_fee_updates {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    keys
}

pub(super) fn insert_resolved_token_address(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    candidates: &mut BTreeSet<String>,
    address: Address,
) {
    insert_resolved_token_address_str(registry, token_index, candidates, &address_string(&address));
}

pub(super) fn insert_resolved_token_address_str(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    candidates: &mut BTreeSet<String>,
    address: &str,
) {
    if let Some(token_address) = token_index.resolve_token_address(address) {
        if registry.token(token_address).is_some() {
            candidates.insert(token_address.to_string());
        }
        return;
    }

    let address = normalize_address(address);
    if registry.token(&address).is_some() {
        candidates.insert(address);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use alloy_primitives::{address, b256, U256};
    use std::cell::RefCell;
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;
    use tx_processor::tx_processor::data_models::{
        ERC20TransferEvent, UniswapV2SwapEvent, UniswapV2SyncEvent,
    };

    use crate::chain_metadata::{
        UniswapV2PoolIdentity, UniswapV2PoolIdentityProvider, UniswapV2PoolMetadataLookup,
    };
    use crate::erc20::ERC20TokenMetadata;

    #[derive(Clone)]
    struct CountingIdentityProvider {
        lookups: Rc<RefCell<usize>>,
    }

    impl UniswapV2PoolIdentityProvider for CountingIdentityProvider {
        fn uniswap_v2_pool_identity<'a>(
            &'a self,
            _lookup: &'a UniswapV2PoolMetadataLookup,
        ) -> Pin<Box<dyn Future<Output = Result<Option<UniswapV2PoolIdentity>>> + 'a>> {
            Box::pin(async move {
                *self.lookups.borrow_mut() += 1;
                Ok(None)
            })
        }
    }

    fn registry_with_token() -> (TokenRegistry, TrackedTokenIndex) {
        let mut registry = TokenRegistry::new();
        registry.add_token(ERC20TokenMetadata::new(
            "0x1111111111111111111111111111111111111111",
            "Token",
            "TKN",
            18,
            "1000",
        ));
        let index = TrackedTokenIndex::from_registry(&registry, 100);
        (registry, index)
    }

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            0,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            None,
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        )
    }

    #[test]
    fn candidates_use_erc20_contracts() {
        let (registry, index) = registry_with_token();
        let mut tx = tx();
        tx.erc20_contracts
            .insert(address!("1111111111111111111111111111111111111111"));

        let candidates = candidate_token_addresses(&registry, &index, &tx);

        assert_eq!(
            candidates,
            vec!["0x1111111111111111111111111111111111111111".to_string()]
        );
    }

    #[test]
    fn candidates_route_from_broad_unique_addresses_for_compatibility() {
        let (registry, index) = registry_with_token();
        let mut tx = tx();
        tx.unique_addresses
            .insert(address!("1111111111111111111111111111111111111111"));

        let candidates = candidate_token_addresses(&registry, &index, &tx);

        assert_eq!(
            candidates,
            vec!["0x1111111111111111111111111111111111111111".to_string()]
        );
    }

    #[tokio::test]
    async fn v2_pool_events_route_from_pair_touching_transfer_before_identity_lookup() {
        let (registry, index) = registry_with_token();
        let mut tx = tx();
        let pair = address!("3333333333333333333333333333333333333333");
        tx.uniswap_v2_swaps.push(UniswapV2SwapEvent {
            pair_address: pair,
            sender: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            to: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            amount0_in: U256::from(1_000_u64),
            amount1_in: U256::ZERO,
            amount0_out: U256::ZERO,
            amount1_out: U256::from(1_000_u64),
            log_index: 1,
        });
        tx.uniswap_v2_syncs.push(UniswapV2SyncEvent {
            pair_address: pair,
            reserve0: U256::from(1_000_u64),
            reserve1: U256::from(1_000_u64),
            log_index: 2,
        });
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: address!("1111111111111111111111111111111111111111"),
            from_address: pair,
            to_address: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            amount: U256::from(1_000_u64),
            log_index: 3,
        });

        let lookups = Rc::new(RefCell::new(0));
        let provider = CountingIdentityProvider {
            lookups: lookups.clone(),
        };
        let candidates =
            candidate_token_addresses_with_pool_discovery(&registry, &index, &tx, &provider, None)
                .await
                .unwrap();

        assert_eq!(
            candidates,
            vec!["0x1111111111111111111111111111111111111111".to_string()]
        );
        assert_eq!(*lookups.borrow(), 0);
    }
}
