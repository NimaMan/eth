use std::collections::BTreeSet;

use tx_processor::ProcessedTransaction;

use crate::network::{
    graph::RawTokenNetworkGraph, ingest::extract_token_network_updates, model::TokenNetworkId,
};
use crate::tracking::{normalize_address, TrackedTokenIndexUpdate};

use super::processor::BlockTokenProcessor;

impl BlockTokenProcessor {
    pub(in crate::tracking::block_processor) fn apply_network_updates_for_transaction(
        &mut self,
        tx: &ProcessedTransaction,
        token_addresses: BTreeSet<String>,
    ) {
        if !self.network_graphs_enabled {
            return;
        }

        for token_address in token_addresses {
            let Some((token_address, decimals)) = self
                .registry
                .token(&token_address)
                .map(|token| (normalize_address(&token.contract_address), token.decimals))
            else {
                self.network_graphs
                    .remove(&normalize_address(&token_address));
                continue;
            };

            let batch = extract_token_network_updates(tx, &token_address, decimals);
            if batch.is_empty() {
                continue;
            }
            self.network_graph_mut(&token_address).apply_batch(batch);
        }
    }

    fn network_graph_mut(&mut self, token_address: &str) -> &mut RawTokenNetworkGraph {
        let address = normalize_address(token_address);
        self.network_graphs
            .entry(address.clone())
            .or_insert_with(|| {
                RawTokenNetworkGraph::new(TokenNetworkId::new(None, address.as_str()))
            })
    }

    pub(in crate::tracking::block_processor) fn cleanup_network_graphs_for_index_update(
        &mut self,
        update: &TrackedTokenIndexUpdate,
    ) {
        if update.removed_by_retention {
            self.network_graphs
                .remove(&normalize_address(&update.token_address));
        }
        if let Some(evicted_token_address) = &update.evicted_token_address {
            self.network_graphs
                .remove(&normalize_address(evicted_token_address));
        }
    }

    pub(in crate::tracking::block_processor) fn cleanup_network_graphs_to_registry(&mut self) {
        self.network_graphs
            .retain(|token_address, _| self.registry.token(token_address).is_some());
    }
}
