//! Conversion from token/pool/transaction state into network updates.

pub mod authority;
pub mod pools;
pub mod transaction;
pub mod transfers;

pub use authority::extract_authority_updates;
pub use pools::extract_pool_updates;
pub use transaction::{
    extract_transaction_updates, AddressCostIngest, AddressMovementIngest, NetworkIngestBatch,
    NodeLabelIngest, TransactionIngestContext,
};
pub use transfers::extract_transfer_updates;

use tx_processor::ProcessedTransaction;

/// Extract all currently supported token-network updates from a transaction.
pub fn extract_token_network_updates(
    tx: &ProcessedTransaction,
    tracked_token_address: impl AsRef<str>,
    token_decimals: u8,
) -> NetworkIngestBatch {
    let tracked_token_address = tracked_token_address.as_ref();
    let mut batch = extract_transaction_updates(tx);
    batch.extend(extract_transfer_updates(
        tx,
        tracked_token_address,
        token_decimals,
    ));
    batch.extend(extract_authority_updates(tx, tracked_token_address));
    batch.extend(extract_pool_updates(tx, tracked_token_address));
    batch
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256, Address, U256};
    use tx_processor::tx_processor::data_models::{
        ERC20TransferEvent, OwnershipTransferredEvent, UniswapV2PairCreatedEvent,
    };

    fn tx() -> ProcessedTransaction {
        ProcessedTransaction::new(
            b256!("0000000000000000000000000000000000000000000000000000000000000001"),
            100,
            1_700,
            3,
            address!("1111111111111111111111111111111111111111"),
            None,
            U256::ZERO,
            true,
            7,
            2,
            Vec::new(),
        )
    }

    #[test]
    fn combined_ingest_extracts_supported_update_families() {
        let token = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let mut tx = tx();
        tx.erc20_transfers.push(ERC20TransferEvent {
            token_address: token,
            from_address: address!("1111111111111111111111111111111111111111"),
            to_address: address!("2222222222222222222222222222222222222222"),
            amount: U256::from(1_000_000_u64),
            log_index: 1,
        });
        tx.ownership_transferred_events
            .push(OwnershipTransferredEvent {
                contract_address: token,
                previous_owner: address!("1111111111111111111111111111111111111111"),
                new_owner: address!("3333333333333333333333333333333333333333"),
                log_index: 2,
            });
        tx.uniswap_v2_pair_created_events
            .push(UniswapV2PairCreatedEvent {
                pair_address: address!("9999999999999999999999999999999999999999"),
                token0: token,
                token1: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                factory_address: Address::ZERO,
                log_index: 3,
            });

        let batch = extract_token_network_updates(
            &tx,
            crate::network::ingest::transaction::address_string(&token),
            6,
        );

        assert_eq!(batch.address_movements.len(), 2);
        assert!(batch.node_labels.len() >= 2);
        assert!(batch.edges.len() >= 3);
    }
}
