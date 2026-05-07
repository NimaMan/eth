use std::collections::BTreeSet;

use tx_processor::ProcessedTransaction;

use crate::erc20::ERC20Token;
use crate::manager::{address_string, normalize_address, same_address_str, TokenRegistry};

pub(crate) fn token_prior_lookup_addresses(
    registry: &TokenRegistry,
    tx: &ProcessedTransaction,
) -> Vec<String> {
    let mut addresses = BTreeSet::new();

    for address in token_event_addresses(tx) {
        if registry.token(&address).is_some() {
            addresses.insert(address);
        }
    }

    for pool_address in pool_lookup_addresses_from_tx(tx) {
        for token in registry.tokens.values() {
            if token
                .pool_addresses()
                .iter()
                .any(|known_pool| known_pool == &pool_address)
            {
                addresses.insert(normalize_address(&token.contract_address));
            }
        }
    }

    addresses.into_iter().collect()
}

pub(crate) fn pool_prior_lookup_addresses(tx: &ProcessedTransaction) -> Vec<String> {
    pool_lookup_addresses_from_tx(tx).into_iter().collect()
}

pub(crate) fn token_state_prior_addresses(
    registry: &TokenRegistry,
    tx: &ProcessedTransaction,
) -> Vec<String> {
    let mut addresses = BTreeSet::new();

    for address in token_control_event_addresses(tx) {
        if registry.token(&address).is_some() {
            addresses.insert(address);
        }
    }

    if tx_action_indicates_token_control(tx) {
        for token in registry.tokens.values() {
            if tx_directly_touches_token(tx, token)
                && tx_touches_any_token_control_address(tx, token)
            {
                addresses.insert(normalize_address(&token.contract_address));
            }
        }
    }

    addresses.into_iter().collect()
}

pub(crate) fn pool_state_prior_addresses(tx: &ProcessedTransaction) -> Vec<String> {
    let mut addresses = BTreeSet::new();

    for event in &tx.uniswap_v2_pair_created_events {
        addresses.insert(address_string(&event.pair_address));
    }
    for event in &tx.uniswap_v2_mints {
        addresses.insert(address_string(&event.pair_address));
    }
    for event in &tx.uniswap_v2_burns {
        addresses.insert(address_string(&event.pair_address));
    }

    addresses.into_iter().collect()
}

pub(crate) fn tx_is_token_control_replay_candidate(
    token: &ERC20Token,
    tx: &ProcessedTransaction,
) -> bool {
    let token_address = normalize_address(&token.contract_address);
    token_control_event_addresses(tx).contains(&token_address)
        || (tx_action_indicates_token_control(tx)
            && tx_directly_touches_token(tx, token)
            && tx_touches_any_token_control_address(tx, token))
}

fn token_event_addresses(tx: &ProcessedTransaction) -> BTreeSet<String> {
    let mut addresses = BTreeSet::new();

    addresses.extend(tx.erc20_contracts.iter().map(address_string));
    for transfer in &tx.erc20_transfers {
        addresses.insert(address_string(&transfer.token_address));
    }
    for approval in &tx.erc20_approval_events {
        addresses.insert(address_string(&approval.token_address));
    }

    addresses.extend(token_control_event_addresses(tx));

    addresses
}

fn token_control_event_addresses(tx: &ProcessedTransaction) -> BTreeSet<String> {
    let mut addresses = BTreeSet::new();

    for event in &tx.trading_enabled_events {
        addresses.insert(address_string(&event.token_address));
    }
    for event in &tx.trading_disabled_events {
        addresses.insert(address_string(&event.token_address));
    }
    for event in &tx.ownership_transferred_events {
        addresses.insert(address_string(&event.contract_address));
    }
    for event in &tx.ownership_transfer_started_events {
        addresses.insert(address_string(&event.contract_address));
    }
    for event in &tx.access_control_role_granted_events {
        addresses.insert(address_string(&event.contract_address));
    }
    for event in &tx.access_control_role_revoked_events {
        addresses.insert(address_string(&event.contract_address));
    }
    for event in &tx.proxy_admin_changed_events {
        addresses.insert(address_string(&event.contract_address));
    }
    if let Some(address) = tx.contract_address {
        addresses.insert(address_string(&address));
    }
    for event in &tx.contract_creation_events {
        addresses.insert(address_string(&event.contract_address));
    }

    addresses
}

fn pool_lookup_addresses_from_tx(tx: &ProcessedTransaction) -> BTreeSet<String> {
    let mut addresses = BTreeSet::new();

    for event in &tx.uniswap_v2_syncs {
        addresses.insert(address_string(&event.pair_address));
    }
    for event in &tx.uniswap_v2_swaps {
        addresses.insert(address_string(&event.pair_address));
    }
    for event in &tx.uniswap_v2_mints {
        addresses.insert(address_string(&event.pair_address));
    }
    for event in &tx.uniswap_v2_burns {
        addresses.insert(address_string(&event.pair_address));
    }
    for event in &tx.uniswap_v2_pair_created_events {
        addresses.insert(address_string(&event.pair_address));
    }

    addresses
}

fn tx_directly_touches_token(tx: &ProcessedTransaction, token: &ERC20Token) -> bool {
    let token_address = normalize_address(&token.contract_address);
    tx.to_address
        .is_some_and(|address| same_address_str(address, &token_address))
        || tx
            .contract_address
            .is_some_and(|address| same_address_str(address, &token_address))
        || tx
            .erc20_contracts
            .iter()
            .any(|address| same_address_str(*address, &token_address))
        || tx
            .unique_addresses
            .iter()
            .any(|address| same_address_str(*address, &token_address))
}

fn tx_touches_any_token_control_address(tx: &ProcessedTransaction, token: &ERC20Token) -> bool {
    let control_addresses = tx_control_addresses(tx);
    token
        .token_control_addresses
        .iter()
        .any(|address| control_addresses.contains(&normalize_address(address)))
}

fn tx_control_addresses(tx: &ProcessedTransaction) -> BTreeSet<String> {
    let mut addresses = BTreeSet::new();
    addresses.extend(tx.unique_addresses.iter().map(address_string));
    addresses.extend(tx.erc20_contracts.iter().map(address_string));
    addresses.insert(address_string(&tx.from_address));
    if let Some(address) = tx.to_address {
        addresses.insert(address_string(&address));
    }
    if let Some(address) = tx.contract_address {
        addresses.insert(address_string(&address));
    }
    addresses
}

fn tx_action_indicates_token_control(tx: &ProcessedTransaction) -> bool {
    tx.actions.iter().any(|action| {
        let action = action.to_ascii_lowercase();
        action.contains("trading")
            || action.contains("tax")
            || action.contains("fee")
            || action.contains("owner")
            || action.contains("ownership")
            || action.contains("control")
            || action.contains("role")
    })
}
