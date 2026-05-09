use alloy_primitives::Address;
use reth_chain_query::reth_index::AddressParticipation;

use crate::ProcessedBlock;

pub fn address_participations_from_processed_block(
    block: &ProcessedBlock,
) -> Vec<AddressParticipation> {
    block
        .transactions
        .iter()
        .filter_map(|tx| {
            let mut addresses = tx
                .processed
                .unique_addresses
                .iter()
                .copied()
                .collect::<Vec<Address>>();

            addresses.push(tx.processed.from_address);
            if let Some(address) = tx.processed.to_address {
                addresses.push(address);
            }
            if let Some(address) = tx.processed.contract_address {
                addresses.push(address);
            }

            if addresses.is_empty() {
                None
            } else {
                Some(AddressParticipation {
                    tx_index: tx.processed.tx_index,
                    addresses,
                })
            }
        })
        .collect()
}
