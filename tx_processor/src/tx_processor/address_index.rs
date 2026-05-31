use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use alloy_primitives::Address;
use serde_json::json;

use super::data_models::ProcessedTransaction;

fn insert_if_some(set: &mut HashSet<Address>, value: Option<Address>) {
    if let Some(addr) = value {
        set.insert(addr);
    }
}

pub(super) fn populate_unique_addresses(tx: &mut ProcessedTransaction) {
    let mut set = HashSet::new();

    set.insert(tx.from_address);
    insert_if_some(&mut set, tx.to_address);
    insert_if_some(&mut set, tx.contract_address);

    for addr in tx.erc20_contracts.iter().copied() {
        set.insert(addr);
    }

    for transfer in &tx.eth_transfers {
        set.insert(transfer.from_address);
        set.insert(transfer.to_address);
    }

    for transfer in &tx.erc20_transfers {
        set.insert(transfer.token_address);
        set.insert(transfer.from_address);
        set.insert(transfer.to_address);
    }

    for call in &tx.internal_erc20_calls {
        set.insert(call.token_address);
        set.insert(call.caller);
        set.insert(call.from_address);
        set.insert(call.to_address);
    }

    for transfer in &tx.internal_erc20_transfers {
        set.insert(transfer.token_address);
        set.insert(transfer.caller);
        set.insert(transfer.from_address);
        set.insert(transfer.to_address);
    }

    for transfer in &tx.erc721_transfers {
        set.insert(transfer.token_address);
        set.insert(transfer.from_address);
        set.insert(transfer.to_address);
    }

    for transfer in &tx.erc1155_transfers {
        set.insert(transfer.token_address);
        set.insert(transfer.operator);
        set.insert(transfer.from_address);
        set.insert(transfer.to_address);
    }

    for approval in &tx.erc20_approval_events {
        set.insert(approval.token_address);
        set.insert(approval.owner);
        set.insert(approval.spender);
    }

    for approval in &tx.erc721_approval_events {
        set.insert(approval.token_address);
        set.insert(approval.owner);
        set.insert(approval.approved_address);
    }

    for approval in &tx.approval_for_all_events {
        set.insert(approval.token_address);
        set.insert(approval.owner);
        set.insert(approval.operator);
    }

    for event in &tx.uniswap_v2_syncs {
        set.insert(event.pair_address);
    }

    for event in &tx.uniswap_v2_swaps {
        set.insert(event.pair_address);
        set.insert(event.sender);
        set.insert(event.to);
    }

    for pool in &tx.uniswap_v3_pools {
        if !pool.factory_address.is_zero() {
            set.insert(pool.factory_address);
        }
        set.insert(pool.pool);
        set.insert(pool.token0);
        set.insert(pool.token1);
    }

    for event in &tx.uniswap_v3_initializations {
        set.insert(event.pool_address);
    }

    for event in &tx.uniswap_v3_mints {
        set.insert(event.pool_address);
        set.insert(event.sender);
        set.insert(event.owner);
    }

    for event in &tx.uniswap_v3_burns {
        set.insert(event.pool_address);
        set.insert(event.owner);
    }

    for event in &tx.uniswap_v3_swaps {
        set.insert(event.pool_address);
        set.insert(event.sender);
        set.insert(event.recipient);
    }

    for event in &tx.uniswap_v3_positions {
        set.insert(event.pool_address);
        set.insert(event.owner);
    }

    for event in &tx.uniswap_v3_increases {
        set.insert(event.pool_address);
    }

    for event in &tx.uniswap_v3_decreases {
        set.insert(event.pool_address);
    }

    for event in &tx.uniswap_v4_initializes {
        set.insert(event.pool_manager_address);
        set.insert(event.currency0);
        set.insert(event.currency1);
        set.insert(event.hooks);
    }

    for event in &tx.uniswap_v4_modifies {
        set.insert(event.pool_manager_address);
        set.insert(event.sender);
    }

    for event in &tx.uniswap_v4_swaps {
        set.insert(event.pool_manager_address);
        set.insert(event.sender);
    }

    for event in &tx.uniswap_v4_donates {
        set.insert(event.pool_manager_address);
        set.insert(event.sender);
    }

    for event in &tx.uniswap_v4_protocol_fee_updates {
        set.insert(event.pool_manager_address);
    }

    for event in &tx.uniswap_v4_dynamic_lp_fee_updates {
        set.insert(event.pool_manager_address);
    }

    for event in &tx.uniswap_v4_protocol_fee_controller_updates {
        set.insert(event.pool_manager_address);
        set.insert(event.protocol_fee_controller);
    }

    for event in &tx.uniswap_v4_balance_deltas {
        set.insert(event.pool_manager_address);
        set.insert(event.settler);
    }

    for event in &tx.permit2_events {
        set.insert(event.pool_manager_address);
        set.insert(event.owner);
        set.insert(event.token);
        set.insert(event.spender);
    }

    for event in &tx.trading_enabled_events {
        set.insert(event.token_address);
    }

    for event in &tx.trading_disabled_events {
        set.insert(event.token_address);
    }

    for event in &tx.uniswap_v2_pair_created_events {
        set.insert(event.pair_address);
        set.insert(event.token0);
        set.insert(event.token1);
    }

    for event in &tx.ownership_transferred_events {
        set.insert(event.contract_address);
        set.insert(event.previous_owner);
        set.insert(event.new_owner);
    }

    for event in &tx.ownership_transfer_started_events {
        set.insert(event.contract_address);
        set.insert(event.previous_owner);
        set.insert(event.new_owner);
    }

    for event in &tx.access_control_role_granted_events {
        set.insert(event.contract_address);
        set.insert(event.account);
        set.insert(event.sender);
    }

    for event in &tx.access_control_role_revoked_events {
        set.insert(event.contract_address);
        set.insert(event.account);
        set.insert(event.sender);
    }

    for event in &tx.proxy_admin_changed_events {
        set.insert(event.contract_address);
        set.insert(event.previous_admin);
        set.insert(event.new_admin);
    }

    for event in &tx.contract_creation_events {
        set.insert(event.contract_address);
    }

    for event in &tx.deposit_events {
        insert_if_some(&mut set, event.token_address);
        insert_if_some(&mut set, event.withdrawal_address);
        insert_if_some(&mut set, event.pair_address);
        insert_if_some(&mut set, event.sender);
    }

    for event in &tx.withdraw_events {
        set.insert(event.pair_address);
        insert_if_some(&mut set, event.sender);
    }

    for event in &tx.uniswap_v2_mints {
        set.insert(event.pair_address);
        set.insert(event.sender);
    }

    for event in &tx.uniswap_v2_burns {
        set.insert(event.pair_address);
        set.insert(event.sender);
    }

    for internal in &tx.internal_transactions {
        set.insert(internal.from_address);
        insert_if_some(&mut set, internal.to_address);
    }

    for address in tx.address_balance_changes.keys() {
        set.insert(*address);
    }

    for address in tx.latest_states.keys() {
        set.insert(*address);
    }

    for event in &tx.other_events {
        if let Some(addr_str) = event.get("address").and_then(|value| value.as_str()) {
            if let Ok(addr) = Address::from_str(addr_str) {
                set.insert(addr);
            }
        }

        if let Some(addresses_value) = event.get("addresses") {
            if let Some(array) = addresses_value.as_array() {
                for addr_value in array {
                    if let Some(addr_str) = addr_value.as_str() {
                        if let Ok(addr) = Address::from_str(addr_str) {
                            set.insert(addr);
                        }
                    }
                }
            }
        }
    }

    tx.unique_addresses = set;
}

pub(super) fn extract_candidate_addresses_from_log(
    log: &alloy_primitives::Log,
) -> HashSet<Address> {
    let mut addresses = HashSet::new();
    if log.address != Address::ZERO {
        addresses.insert(log.address);
    }

    for topic in log.topics() {
        let bytes: &[u8] = topic.as_ref();
        if bytes.len() == 32 && bytes[..12].iter().all(|b| *b == 0) {
            let candidate = Address::from_slice(&bytes[12..]);
            if candidate != Address::ZERO {
                addresses.insert(candidate);
            }
        }
    }

    let data_bytes = log.data.data.as_ref();
    for chunk in data_bytes.chunks(32) {
        if chunk.len() == 32 && chunk[..12].iter().all(|b| *b == 0) {
            let candidate = Address::from_slice(&chunk[12..]);
            if candidate != Address::ZERO {
                addresses.insert(candidate);
            }
        }
    }

    addresses
}

pub(super) fn build_unknown_event_record(
    log: &alloy_primitives::Log,
    log_index: u64,
    event_type: Option<&str>,
    candidate_addresses: &HashSet<Address>,
) -> HashMap<String, serde_json::Value> {
    let mut record = HashMap::new();
    record.insert("address".to_string(), json!(format!("{:#x}", log.address)));
    record.insert("log_index".to_string(), json!(log_index));
    if let Some(topic0) = log.topics().first() {
        record.insert(
            "event_signature".to_string(),
            json!(format!("{:#x}", topic0)),
        );
    }
    let topics: Vec<String> = log.topics().iter().map(|t| format!("{:#x}", t)).collect();
    record.insert("topics".to_string(), json!(topics));
    record.insert(
        "data".to_string(),
        json!(format!("0x{}", hex::encode(log.data.data.as_ref()))),
    );
    if let Some(label) = event_type {
        record.insert("event_type".to_string(), json!(label));
    }
    if !candidate_addresses.is_empty() {
        let mut addr_list: Vec<String> = candidate_addresses
            .iter()
            .copied()
            .filter(|addr| *addr != Address::ZERO)
            .map(|addr| format!("{:#x}", addr))
            .collect();
        addr_list.sort();
        record.insert("addresses".to_string(), json!(addr_list));
    }
    record
}
