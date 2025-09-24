use std::sync::Arc;

use alloy_primitives::{Address, B256};
use eyre::Result;
use tokio::sync::Mutex;

use crate::block_processor::ProcessedBlock;
use crate::processed_tx_provider::cache::ProcessedBlockCache;
use crate::processed_tx_provider::ProcessedTxProvider;
use crate::tx_processor::data_models::ProcessedTransaction;

/// Provider that materialises processed transactions for a specific ERC20 token.
pub struct TokenProcessedTxProvider {
    core: Arc<ProcessedTxProvider>,
    query_provider: Arc<reth_chain_query::RethQueryProvider>,
    cache: Arc<Mutex<ProcessedBlockCache>>,
}

impl TokenProcessedTxProvider {
    pub fn new(core: Arc<ProcessedTxProvider>) -> Result<Self> {
        let query_provider = Arc::new(reth_chain_query::RethQueryProvider::with_provider_factory(
            Arc::new(core.provider_factory.clone()),
        )?);

        Ok(Self {
            core,
            query_provider,
            cache: Arc::new(Mutex::new(ProcessedBlockCache::new())),
        })
    }

    /// Load processed blocks touching the token contract between `start_block` and `end_block` (inclusive).
    pub async fn load_blocks_for_token(
        &self,
        token: Address,
        start_block: u64,
        end_block: u64,
    ) -> Result<()> {
        if end_block < start_block {
            return Ok(());
        }

        let mut blocks = self
            .query_provider
            .get_address_account_history_blocks(token, start_block, end_block)
            .await?;

        if blocks.is_empty() {
            blocks = (start_block..=end_block).collect();
        }

        self.ensure_blocks_cached(&blocks).await?;
        Ok(())
    }

    /// Return processed transactions that involve the token within the currently cached blocks.
    pub async fn transactions_for(&self, token: Address) -> Vec<ProcessedTransaction> {
        let cache = self.cache.lock().await;
        cache
            .iter()
            .flat_map(|(_, block)| filter_transactions_for_token(block, token))
            .collect()
    }

    async fn ensure_blocks_cached(&self, block_numbers: &[u64]) -> Result<()> {
        let missing = {
            let cache = self.cache.lock().await;
            cache.missing_blocks(block_numbers)
        };

        if missing.is_empty() {
            return Ok(());
        }

        let processed_blocks = self.core.process_block_batch(missing, None).await?;

        let mut cache = self.cache.lock().await;
        cache.insert_many(processed_blocks);
        Ok(())
    }

    /// Retrieve a transaction by hash, checking the cache first and falling back to the core provider.
    pub async fn transaction_by_hash(&self, tx_hash: B256) -> Result<ProcessedTransaction> {
        if let Some(tx) = self.cache.lock().await.get_transaction(&tx_hash) {
            return Ok(tx);
        }

        self.core.process_transaction_by_hash(tx_hash).await
    }
}

fn filter_transactions_for_token(
    block: &Arc<ProcessedBlock>,
    token: Address,
) -> Vec<ProcessedTransaction> {
    block
        .transactions
        .iter()
        .filter(|tx| transaction_involves_token(&tx.processed, token))
        .map(|tx| tx.processed.clone())
        .collect()
}

fn transaction_involves_token(tx: &ProcessedTransaction, token: Address) -> bool {
    tx.erc20_transfers.iter().any(|t| t.token_address == token)
        || tx.approvals.iter().any(|a| a.token_address == token)
        || tx.erc20_contracts.contains(&token)
}
