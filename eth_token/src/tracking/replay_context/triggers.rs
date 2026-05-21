use std::collections::BTreeSet;

use tx_processor::ProcessedTransaction;

use crate::erc20::ERC20Token;
use crate::tracking::{address_string, normalize_address, same_address_str};

pub(crate) fn tx_is_token_control_replay_candidate(
    token: &ERC20Token,
    tx: &ProcessedTransaction,
) -> bool {
    let token_address = normalize_address(&token.contract_address);
    token_control_event_addresses(tx).contains(&token_address)
        || (tx_directly_touches_token(tx, token)
            && tx_touches_any_token_control_address(tx, token)
            && (tx_action_indicates_token_control(tx) || tx_direct_call_to_token(tx, token)))
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

fn tx_direct_call_to_token(tx: &ProcessedTransaction, token: &ERC20Token) -> bool {
    let token_address = normalize_address(&token.contract_address);
    tx.to_address
        .is_some_and(|address| same_address_str(address, &token_address))
        && !tx.input.is_empty()
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

#[cfg(test)]
mod tests {
    use alloy_primitives::{address, b256, U256};
    use tx_processor::ProcessedTransaction;

    use crate::erc20::{ERC20Token, ERC20TokenMetadata};

    use super::*;

    fn token_with_control() -> ERC20Token {
        let mut token = ERC20Token::new(ERC20TokenMetadata::new(
            "0x1111111111111111111111111111111111111111",
            "Token",
            "TKN",
            18,
            "1000000000000000000",
        ));
        token
            .token_control_addresses
            .insert("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string());
        token
    }

    #[test]
    fn direct_control_call_to_token_forces_replay_without_action_keyword() {
        let token = token_with_control();
        let tx = ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            10,
            1000,
            0,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            Some(address!("1111111111111111111111111111111111111111")),
            U256::ZERO,
            true,
            0,
            2,
            vec![0x30, 0x1e, 0x57, 0x46],
        );

        assert!(tx_is_token_control_replay_candidate(&token, &tx));
    }
}
