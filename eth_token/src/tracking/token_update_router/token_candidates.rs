use std::collections::BTreeSet;
use std::time::Duration;

use alloy_primitives::Address;
use eyre::Result;
use tx_processor::ProcessedTransaction;

use crate::chain_metadata::{UniswapV2PoolMetadataLookup, UniswapV2PoolMetadataProvider};
use crate::pools::uniswap::v4_event_display_key;
use crate::tracking::{address_string, normalize_address, TokenRegistry, TrackedTokenIndex};

use super::pool_metadata_lookup::optional_uniswap_v2_pool_metadata;

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
    P: UniswapV2PoolMetadataProvider,
{
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

    for pool_address in v2_pool_event_addresses(tx) {
        let pool_address_string = address_string(&pool_address);
        if token_index
            .resolve_token_address(&pool_address_string)
            .is_some()
        {
            continue;
        }

        let Some(metadata) = optional_uniswap_v2_pool_metadata(
            pool_metadata_provider,
            UniswapV2PoolMetadataLookup {
                tracked_token_address: None,
                pool_address,
                block_number: tx.block_number,
                transaction_hash: tx.hash,
                tx_index: tx.tx_index,
            },
            metadata_timeout,
        )
        .await?
        else {
            continue;
        };

        insert_resolved_token_address_str(registry, token_index, &mut candidates, &metadata.token0);
        insert_resolved_token_address_str(registry, token_index, &mut candidates, &metadata.token1);
    }

    Ok(candidates.into_iter().collect())
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
