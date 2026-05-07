use std::collections::HashMap;

use tx_processor::ProcessedTransaction;

#[derive(Clone, Debug, Default)]
pub(crate) struct TokenPriorIndex {
    prior_txs_by_token: HashMap<String, Vec<ProcessedTransaction>>,
}

impl TokenPriorIndex {
    pub(crate) fn record_token_prior(&mut self, token_address: String, tx: ProcessedTransaction) {
        let prior_txs = self.prior_txs_by_token.entry(token_address).or_default();
        if prior_txs.iter().any(|prior_tx| prior_tx.hash == tx.hash) {
            return;
        }
        prior_txs.push(tx);
        prior_txs.sort_by_key(|tx| tx.tx_index);
    }

    pub(crate) fn priors_for_token(&self, token_address: &str) -> Vec<ProcessedTransaction> {
        self.prior_txs_by_token
            .get(token_address)
            .cloned()
            .unwrap_or_default()
    }
}
