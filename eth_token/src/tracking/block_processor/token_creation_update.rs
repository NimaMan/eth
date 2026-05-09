use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use alloy_primitives::{Address, B256};
use tx_processor::ProcessedTransaction;

use crate::chain_metadata::{TokenMetadataLookup, TokenMetadataProvider};
use crate::tracking::{address_string, hash_string, normalize_address, TrackedTokenStatus};

use super::processor::BlockTokenProcessor;

const LIVE_METADATA_LOOKUP_TIMEOUT_MS: u64 = 2_500;
const LIVE_TOKEN_TRACKER_LOG_TARGET: &str = "live_token_tracker";

impl BlockTokenProcessor {
    pub(in crate::tracking::block_processor) async fn discover_created_tokens<P>(
        &mut self,
        tx: &ProcessedTransaction,
        pending_tx_hashes: &[B256],
        metadata_provider: &P,
    ) -> eyre::Result<Vec<String>>
    where
        P: TokenMetadataProvider,
    {
        let mut created = Vec::new();

        for token_address in created_token_addresses(tx) {
            let token_address_string = address_string(&token_address);
            if self.registry.token(&token_address_string).is_some() {
                continue;
            }

            let lookup = TokenMetadataLookup {
                token_address,
                block_number: tx.block_number,
                block_timestamp: tx.block_timestamp,
                metadata_block_number: tx.block_number.saturating_sub(1),
                transaction_hash: tx.hash,
                tx_index: tx.tx_index,
                creator_address: tx.from_address,
                creator_nonce: tx.nonce,
                pending_tx_hashes: pending_tx_hashes.to_vec(),
            };

            let metadata = if self.is_live_mode {
                match tokio::time::timeout(
                    Duration::from_millis(LIVE_METADATA_LOOKUP_TIMEOUT_MS),
                    metadata_provider.token_metadata(&lookup),
                )
                .await
                {
                    Ok(metadata) => metadata?,
                    Err(_) => {
                        tracing::warn!(
                            target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                            block_number = tx.block_number,
                            tx_index = tx.tx_index,
                            tx_hash = %hash_string(&tx.hash),
                            token_address = %token_address_string,
                            timeout_ms = LIVE_METADATA_LOOKUP_TIMEOUT_MS,
                            action = "token_metadata_lookup",
                            result = "timeout",
                            "live token metadata lookup timed out"
                        );
                        None
                    }
                }
            } else {
                metadata_provider.token_metadata(&lookup).await?
            };

            let Some(metadata) = metadata else {
                continue;
            };

            let token_address = normalize_address(&metadata.address);
            if self.registry.token(&token_address).is_some() {
                continue;
            }

            self.registry
                .add_token_with_live_mode(metadata, self.is_live_mode);
            {
                let Some(token) = self.registry.token_mut(&token_address) else {
                    continue;
                };
                token.handle_contract_creation(
                    tx.block_number,
                    tx.block_timestamp,
                    hash_string(&tx.hash),
                    address_string(&tx.from_address),
                    tx.nonce,
                );
            }
            self.index_registry_token(
                &token_address,
                TrackedTokenStatus::Creation,
                tx.block_number,
            );
            created.push(token_address);
        }

        Ok(created)
    }
}

fn created_token_addresses(tx: &ProcessedTransaction) -> Vec<Address> {
    if !should_apply_transaction_to_token_state(tx) {
        return Vec::new();
    }

    let mut addresses = Vec::new();
    if let Some(address) = tx.contract_address {
        addresses.push(address);
    }
    addresses.extend(
        tx.contract_creation_events
            .iter()
            .map(|event| event.contract_address),
    );
    addresses.sort();
    addresses.dedup();
    addresses
}

pub(in crate::tracking::block_processor) fn should_apply_transaction_to_token_state(
    tx: &ProcessedTransaction,
) -> bool {
    tx.status
}

pub(in crate::tracking::block_processor) fn index_metadata_transaction(
    index: &mut HashMap<Address, Vec<B256>>,
    tx: &ProcessedTransaction,
) {
    let mut addresses = BTreeSet::new();
    addresses.insert(tx.from_address);
    addresses.extend(tx.unique_addresses.iter().copied());

    for address in addresses {
        index.entry(address).or_default().push(tx.hash);
    }
}

pub(in crate::tracking::block_processor) fn pending_metadata_tx_hashes(
    index: &HashMap<Address, Vec<B256>>,
    tx: &ProcessedTransaction,
) -> Vec<B256> {
    index
        .get(&tx.from_address)
        .cloned()
        .unwrap_or_else(|| vec![tx.hash])
}
