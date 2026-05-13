use alloy_primitives::{Address, I256, U256};
use reth_chain_query::to_checksum_address;

use crate::tx_processor::address_balance_change_calculator::get_token_symbol;
use crate::tx_processor::data_models::ProcessedTransaction;

pub(super) fn extract_denom_received(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
    pool_address: Address,
    denom_address: Address,
    weth_address: Address,
) -> U256 {
    if let Some(recipient_net) =
        recipient_net_denom_received(processed_tx, recipient_address, denom_address)
    {
        return recipient_net;
    }

    let gross_pool_output =
        processed_tx
            .erc20_transfers
            .iter()
            .fold(U256::ZERO, |acc, transfer| {
                if transfer.from_address == pool_address
                    && denom_transfer_matches(transfer.token_address, denom_address, weth_address)
                {
                    acc + transfer.amount
                } else {
                    acc
                }
            });
    if !gross_pool_output.is_zero() {
        return gross_pool_output;
    }

    U256::ZERO
}

fn recipient_net_denom_received(
    processed_tx: &ProcessedTransaction,
    recipient_address: Address,
    denom_address: Address,
) -> Option<U256> {
    if let Some(balance_changes) = processed_tx.address_balance_changes.get(&recipient_address) {
        if denom_address.is_zero() {
            if let Some(&amount) = balance_changes.currency_net.get("ETH") {
                if amount > I256::ZERO {
                    return Some(amount.unsigned_abs());
                }
            }
        }

        if let Some(symbol) = get_token_symbol(&denom_address) {
            if let Some(&amount) = balance_changes.currency_net.get(symbol) {
                if amount > I256::ZERO {
                    return Some(amount.unsigned_abs());
                }
            }
            if symbol == "WETH" {
                if let Some(&amount) = balance_changes.currency_net.get("ETH") {
                    if amount > I256::ZERO {
                        return Some(amount.unsigned_abs());
                    }
                }
            }
        }

        let token_key = to_checksum_address(&denom_address);
        if let Some(&amount) = balance_changes.token_net.get(&token_key) {
            if amount > I256::ZERO {
                return Some(amount.unsigned_abs());
            }
        }
    }
    None
}

fn denom_transfer_matches(
    token_address: Address,
    denom_address: Address,
    weth_address: Address,
) -> bool {
    token_address == denom_address
        || ((denom_address.is_zero() || denom_address == weth_address)
            && token_address == weth_address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tx_processor::data_models::{
        AddressBalanceChange, ERC20TransferEvent, ProcessedTransaction,
    };
    use alloy_primitives::{address, B256};
    use std::collections::HashMap;

    #[test]
    fn denom_received_prefers_recipient_net_over_gross_pool_output() {
        let seller = address!("0000000000000000000000000000000000000001");
        let pool = address!("0000000000000000000000000000000000000002");
        let router = address!("0000000000000000000000000000000000000003");
        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let gross_output = U256::from(12345u64);
        let recipient_output = U256::from(6789u64);

        let mut processed = empty_processed_tx(seller);
        processed.erc20_transfers.push(ERC20TransferEvent {
            token_address: weth,
            from_address: pool,
            to_address: router,
            amount: gross_output,
            log_index: 0,
        });
        let mut change = AddressBalanceChange::default();
        change
            .currency_net
            .insert("ETH".to_string(), I256::from_raw(recipient_output));
        processed.address_balance_changes = HashMap::from([(seller, change)]);

        assert_eq!(
            extract_denom_received(&processed, seller, pool, weth, weth),
            recipient_output
        );
    }

    #[test]
    fn denom_received_falls_back_to_gross_pool_output_without_recipient_net() {
        let seller = address!("0000000000000000000000000000000000000001");
        let pool = address!("0000000000000000000000000000000000000002");
        let router = address!("0000000000000000000000000000000000000003");
        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let output = U256::from(12345u64);

        let mut processed = empty_processed_tx(seller);
        processed.erc20_transfers.push(ERC20TransferEvent {
            token_address: weth,
            from_address: pool,
            to_address: router,
            amount: output,
            log_index: 0,
        });

        assert_eq!(
            extract_denom_received(&processed, seller, pool, weth, weth),
            output
        );
    }

    #[test]
    fn denom_received_uses_recipient_net_change() {
        let seller = address!("0000000000000000000000000000000000000001");
        let pool = address!("0000000000000000000000000000000000000002");
        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let output = U256::from(67890u64);

        let mut processed = empty_processed_tx(seller);
        let mut change = AddressBalanceChange::default();
        change
            .currency_net
            .insert("ETH".to_string(), I256::from_raw(output));
        processed.address_balance_changes = HashMap::from([(seller, change)]);

        assert_eq!(
            extract_denom_received(&processed, seller, pool, weth, weth),
            output
        );
    }

    fn empty_processed_tx(sender: Address) -> ProcessedTransaction {
        ProcessedTransaction::new(
            B256::ZERO,
            0,
            0,
            0,
            sender,
            None,
            U256::ZERO,
            true,
            0,
            0,
            Vec::new(),
        )
    }
}
