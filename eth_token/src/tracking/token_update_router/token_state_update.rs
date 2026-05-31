use tx_processor::ProcessedTransaction;

use super::token_candidates::v4_pool_event_keys;
use crate::tracking::{address_string, normalize_address, same_address_str};

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
    tx.to_address
        .is_some_and(|address| same_address_str(address, token_address))
        || tx
            .latest_states
            .keys()
            .any(|address| same_address_str(*address, token_address))
        || tx
            .address_balance_changes
            .keys()
            .any(|address| same_address_str(*address, token_address))
        || other_event_touches_address(tx, token_address)
        || tx
            .erc20_contracts
            .iter()
            .any(|address| same_address_str(*address, token_address))
        || tx
            .erc20_transfers
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .internal_erc20_calls
            .iter()
            .any(|call| same_address_str(call.token_address, token_address))
        || tx
            .internal_erc20_transfers
            .iter()
            .any(|transfer| same_address_str(transfer.token_address, token_address))
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

fn other_event_touches_address(tx: &ProcessedTransaction, address: &str) -> bool {
    let address = normalize_address(address);
    tx.other_events.iter().any(|event| {
        event
            .get("address")
            .and_then(|value| value.as_str())
            .is_some_and(|value| normalize_address(value) == address)
            || event
                .get("addresses")
                .and_then(|value| value.as_array())
                .is_some_and(|values| {
                    values.iter().any(|value| {
                        value
                            .as_str()
                            .is_some_and(|value| normalize_address(value) == address)
                    })
                })
    })
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{address, b256, U256};
    use serde_json::json;
    use tx_processor::ProcessedTransaction;

    use super::*;

    #[test]
    fn direct_call_to_token_touches_token_state() {
        let token = address!("1111111111111111111111111111111111111111");
        let tx = ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            10,
            1000,
            0,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            Some(token),
            U256::ZERO,
            true,
            0,
            2,
            vec![0x30, 0x1e, 0x57, 0x46],
        );

        assert!(touches_token_state(
            &tx,
            "0x1111111111111111111111111111111111111111"
        ));
    }

    #[test]
    fn unknown_token_event_touches_token_state() {
        let mut tx = ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            10,
            1000,
            0,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            Some(address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")),
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        );
        tx.other_events.push(
            [
                (
                    "address".to_string(),
                    json!("0x1111111111111111111111111111111111111111"),
                ),
                ("event_type".to_string(), json!("unknown")),
            ]
            .into_iter()
            .collect(),
        );

        assert!(touches_token_state(
            &tx,
            "0x1111111111111111111111111111111111111111"
        ));
    }

    #[test]
    fn internal_erc20_call_touches_token_state() {
        use tx_processor::tx_processor::data_models::{Erc20CallKind, InternalErc20Call};

        let token = address!("1111111111111111111111111111111111111111");
        let mut tx = ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            10,
            1000,
            0,
            address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            Some(address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")),
            U256::ZERO,
            true,
            0,
            2,
            Vec::new(),
        );
        tx.internal_erc20_calls.push(InternalErc20Call {
            token_address: token,
            caller: address!("cccccccccccccccccccccccccccccccccccccccc"),
            kind: Erc20CallKind::TransferFrom,
            from_address: address!("dddddddddddddddddddddddddddddddddddddddd"),
            to_address: address!("000000000000000000000000000000000000dEaD"),
            amount: U256::from(1_000_u64),
            depth: 1,
            call_type: Some("CALL".to_string()),
            succeeded: true,
        });

        assert!(touches_token_state(
            &tx,
            "0x1111111111111111111111111111111111111111"
        ));
    }
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
