use alloy_primitives::{Address, I256, U256};

use crate::tx_processor::{
    address_balance_change_calculator::get_token_symbol, data_models::ProcessedTransaction,
};
use reth_chain_query::to_checksum_address;

pub(super) fn extract_token_balance_delta(
    processed_tx: &ProcessedTransaction,
    account: Address,
    token_address: Address,
) -> I256 {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&account) {
        if token_address == Address::ZERO {
            if let Some(&amount) = balance_changes.currency_net.get("ETH") {
                return amount;
            }
        }
        if let Some(symbol) = get_token_symbol(&token_address) {
            if let Some(&amount) = balance_changes.currency_net.get(symbol) {
                return amount;
            }
            if symbol == "WETH" {
                if let Some(&amount) = balance_changes.currency_net.get("ETH") {
                    return amount;
                }
            }
        }
        let token_key = to_checksum_address(&token_address);
        if let Some(&amount) = balance_changes.token_net.get(&token_key) {
            return amount;
        }
    }
    I256::ZERO
}

pub(super) fn extract_tokens_received_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
    token_address: Address,
    _token_decimals: u8,
) -> U256 {
    let delta = extract_token_balance_delta(processed_tx, recipient_address, token_address);
    if delta > I256::ZERO {
        delta.unsigned_abs()
    } else {
        U256::ZERO
    }
}

pub(super) fn extract_denom_received_from_processed_transaction(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
    denom_address: Address,
) -> Option<U256> {
    let delta = extract_token_balance_delta(processed_tx, recipient_address, denom_address);
    if delta > I256::ZERO {
        Some(delta.unsigned_abs())
    } else {
        None
    }
}
