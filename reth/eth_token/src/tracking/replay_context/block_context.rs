use tx_processor::ProcessedTransaction;

use crate::tracking::TokenRegistry;

use super::pool_prior_index::PoolPriorIndex;
use super::token_prior_index::TokenPriorIndex;
use super::triggers::{
    pool_prior_lookup_addresses, pool_state_prior_addresses, token_prior_lookup_addresses,
    token_state_prior_addresses,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct BlockReplayContext {
    token_prior_index: TokenPriorIndex,
    pool_prior_index: PoolPriorIndex,
}

impl BlockReplayContext {
    pub(crate) fn prior_txs_for_transaction(
        &self,
        registry: &TokenRegistry,
        tx: &ProcessedTransaction,
    ) -> Vec<ProcessedTransaction> {
        let mut prior_txs = Vec::new();

        for token_address in token_prior_lookup_addresses(registry, tx) {
            prior_txs.extend(self.token_prior_index.priors_for_token(&token_address));
        }
        for pool_address in pool_prior_lookup_addresses(tx) {
            prior_txs.extend(self.pool_prior_index.priors_for_pool(&pool_address));
        }

        sort_and_dedup_prior_txs(prior_txs)
    }

    pub(crate) fn observe_transaction(
        &mut self,
        registry: &TokenRegistry,
        tx: &ProcessedTransaction,
    ) {
        for token_address in token_state_prior_addresses(registry, tx) {
            self.token_prior_index
                .record_token_prior(token_address, tx.clone());
        }
        for pool_address in pool_state_prior_addresses(tx) {
            self.pool_prior_index
                .record_pool_prior(pool_address, tx.clone());
        }
    }
}

fn sort_and_dedup_prior_txs(mut prior_txs: Vec<ProcessedTransaction>) -> Vec<ProcessedTransaction> {
    prior_txs.sort_by_key(|tx| tx.tx_index);
    prior_txs.dedup_by_key(|tx| tx.hash);
    prior_txs
}
