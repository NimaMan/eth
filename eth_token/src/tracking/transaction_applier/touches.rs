use tx_processor::ProcessedTransaction;

use super::candidates::v4_pool_event_keys;
use crate::tracking::{address_string, same_address_str};

pub(super) fn tx_control_addresses(tx: &ProcessedTransaction) -> Vec<String> {
    let mut addresses: Vec<String> = tx.unique_addresses.iter().map(address_string).collect();
    addresses.push(address_string(&tx.from_address));
    if let Some(to_address) = tx.to_address {
        addresses.push(address_string(&to_address));
    }
    if let Some(contract_address) = tx.contract_address {
        addresses.push(address_string(&contract_address));
    }
    addresses
}

pub(super) fn touches_token_state(tx: &ProcessedTransaction, token_address: &str) -> bool {
    tx.erc20_contracts
        .iter()
        .any(|address| same_address_str(*address, token_address))
        || tx
            .erc20_transfers
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .erc20_approval_events
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .ownership_transferred_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .ownership_transfer_started_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .access_control_role_granted_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .access_control_role_revoked_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .proxy_admin_changed_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .trading_enabled_events
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .trading_disabled_events
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .contract_address
            .is_some_and(|address| same_address_str(address, token_address))
        || tx
            .contract_creation_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
}

pub(super) fn touches_v2_pool(tx: &ProcessedTransaction, pool_address: &str) -> bool {
    tx.uniswap_v2_syncs
        .iter()
        .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_swaps
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_mints
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_burns
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .erc20_transfers
            .iter()
            .any(|event| same_address_str(event.token_address, pool_address))
        || tx
            .erc20_approval_events
            .iter()
            .any(|event| same_address_str(event.token_address, pool_address))
}

pub(super) fn touches_v3_pool(tx: &ProcessedTransaction, pool_address: &str) -> bool {
    tx.uniswap_v3_initializations
        .iter()
        .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_swaps
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_mints
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_burns
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_positions
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_increases
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_decreases
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
}

pub(super) fn touches_v4_pool(tx: &ProcessedTransaction, pool_key: &str) -> bool {
    v4_pool_event_keys(tx).iter().any(|key| key == pool_key)
}
