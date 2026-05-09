use std::collections::HashMap;

use tx_processor::ProcessedTransaction;

#[derive(Clone, Debug, Default)]
pub(crate) struct PoolPriorIndex {
    prior_txs_by_pool: HashMap<String, Vec<ProcessedTransaction>>,
}

impl PoolPriorIndex {
    pub(crate) fn record_pool_prior(&mut self, pool_address: String, tx: ProcessedTransaction) {
        let prior_txs = self.prior_txs_by_pool.entry(pool_address).or_default();
        if prior_txs.iter().any(|prior_tx| prior_tx.hash == tx.hash) {
            return;
        }
        prior_txs.push(tx);
        prior_txs.sort_by_key(|tx| tx.tx_index);
    }

    pub(crate) fn priors_for_pool(&self, pool_address: &str) -> Vec<ProcessedTransaction> {
        self.prior_txs_by_pool
            .get(pool_address)
            .cloned()
            .unwrap_or_default()
    }
}
