use std::sync::Arc;

use alloy_primitives::{Address, B256};
use eyre::Result;
use tokio::sync::Mutex;

use crate::block_processor::ProcessedBlock;
use crate::processed_tx_provider::cache::ProcessedBlockCache;
use crate::processed_tx_provider::ProcessedTxProvider;
use crate::tx_processor::data_models::ProcessedTransaction;

/// Provider that materialises processed transactions for a specific address.
pub struct AddressProcessedTxProvider {
    core: Arc<ProcessedTxProvider>,
    query_provider: Arc<reth_chain_query::RethQueryProvider>,
    cache: Arc<Mutex<ProcessedBlockCache>>,
}

impl AddressProcessedTxProvider {
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

    /// Load processed blocks touching `address` between `start_block` and `end_block` (inclusive).
    pub async fn load_blocks_for_address(
        &self,
        address: Address,
        start_block: u64,
        end_block: u64,
    ) -> Result<()> {
        if end_block < start_block {
            return Ok(());
        }

        let mut blocks = self
            .query_provider
            .get_address_account_history_blocks(address, start_block, end_block)
            .await?;

        if blocks.is_empty() {
            // Fallback: use the raw range if history index is empty.
            blocks = (start_block..=end_block).collect();
        }

        self.ensure_blocks_cached(&blocks).await?;
        Ok(())
    }

    /// Return processed transactions that involve `address` within the currently cached blocks.
    pub async fn transactions_for(&self, address: Address) -> Vec<ProcessedTransaction> {
        let cache = self.cache.lock().await;
        cache
            .iter()
            .flat_map(|(_, block)| filter_transactions_for_address(block, address))
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

fn filter_transactions_for_address(
    block: &Arc<ProcessedBlock>,
    address: Address,
) -> Vec<ProcessedTransaction> {
    block
        .transactions
        .iter()
        .filter(|tx| {
            tx.processed.unique_addresses.contains(&address)
                || tx.processed.from_address == address
                || tx.processed.to_address == Some(address)
        })
        .map(|tx| tx.processed.clone())
        .collect()
}
