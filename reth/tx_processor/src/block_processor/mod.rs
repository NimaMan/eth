mod conversion;
pub mod types;

use crate::tx_processor::data_models::{
    ContractCreationEvent, ProcessedAccessListItem, ProcessedTransaction,
};
use crate::tx_processor::{AddressBalanceChangeCalculator, TransactionTraceProcessor, TxProcessor};
use alloy_primitives::{keccak256, Address, B256};
use eyre::{bail, Result};
use futures::stream::{self, StreamExt, TryStreamExt};
use reth_chain_query::{
    provider::{
        BlockDataFetcher, RawBlockData, RpcBlockDataFetcher, TransactionData, TransactionReceipt,
        TransactionTrace,
    },
    RethQueryProvider,
};
use rlp::RlpStream;
use std::sync::Arc;

pub use types::{BlockBatchOptions, ProcessedBlock, ProcessedBlockTransactions};

/// High-level orchestration for processing entire blocks worth of transactions.
#[derive(Clone)]
pub struct BlockProcessor {
    provider: Option<Arc<RethQueryProvider>>,
    fetcher: Arc<BlockDataFetcher>,
    tx_processor: Arc<TxProcessor>,
}

impl BlockProcessor {
    /// Build a block processor with a shared provider handle.
    pub fn new(provider: Arc<RethQueryProvider>) -> Self {
        let fetcher = Arc::new(BlockDataFetcher::new(provider.clone()));
        Self::with_block_fetcher_impl(Some(provider), fetcher, Arc::new(TxProcessor::new()))
    }

    /// Build a block processor re-using an existing [`TxProcessor`].
    pub fn with_tx_processor(
        provider: Arc<RethQueryProvider>,
        tx_processor: Arc<TxProcessor>,
    ) -> Self {
        let fetcher = Arc::new(BlockDataFetcher::new(provider.clone()));
        Self::with_block_fetcher_impl(Some(provider), fetcher, tx_processor)
    }

    /// Build a block processor with an RPC block fetcher in addition to MDBX.
    pub fn with_rpc_fetcher(
        provider: Arc<RethQueryProvider>,
        rpc_fetcher: RpcBlockDataFetcher,
    ) -> Self {
        let fetcher = BlockDataFetcher::new(provider.clone()).with_rpc_fetcher(rpc_fetcher);
        let fetcher = Arc::new(fetcher);
        Self::with_block_fetcher_impl(Some(provider), fetcher, Arc::new(TxProcessor::new()))
    }

    /// Build a block processor with an explicit block data fetcher handle.
    pub fn with_block_fetcher(fetcher: Arc<BlockDataFetcher>) -> Self {
        let provider = fetcher.provider().cloned();
        Self::with_block_fetcher_impl(provider, fetcher, Arc::new(TxProcessor::new()))
    }

    /// Build a block processor with explicit fetcher and tx-processor handles.
    pub fn with_block_fetcher_and_tx(
        fetcher: Arc<BlockDataFetcher>,
        tx_processor: Arc<TxProcessor>,
    ) -> Self {
        let provider = fetcher.provider().cloned();
        Self::with_block_fetcher_impl(provider, fetcher, tx_processor)
    }

    fn with_block_fetcher_impl(
        provider: Option<Arc<RethQueryProvider>>,
        fetcher: Arc<BlockDataFetcher>,
        tx_processor: Arc<TxProcessor>,
    ) -> Self {
        Self {
            provider,
            fetcher,
            tx_processor,
        }
    }

    /// Access the transaction processor used for decoding.
    pub fn tx_processor(&self) -> &Arc<TxProcessor> {
        &self.tx_processor
    }

    /// Access the underlying block data fetcher.
    pub fn block_fetcher(&self) -> &Arc<BlockDataFetcher> {
        &self.fetcher
    }

    /// Process a single block by number, fetching the raw data first.
    pub async fn process_block(&self, block_number: u64) -> Result<ProcessedBlock> {
        self.process_block_with_options(block_number, true).await
    }

    /// Process a single block with explicit trace inclusion behaviour.
    pub async fn process_block_with_options(
        &self,
        block_number: u64,
        include_traces: bool,
    ) -> Result<ProcessedBlock> {
        let raw = self
            .fetcher
            .fetch_db_block_with_traces(block_number, include_traces)
            .await?;
        self.process_raw_block(raw).await
    }

    /// Access the underlying query provider if configured (MDBX mode only).
    pub fn provider(&self) -> Option<&Arc<RethQueryProvider>> {
        self.provider.as_ref()
    }

    /// Process a block fetched through RPC (useful for live pipelines).
    pub async fn process_block_via_rpc(
        &self,
        block_hash: B256,
        block_number: u64,
        include_traces: bool,
    ) -> Result<ProcessedBlock> {
        let raw = self
            .fetcher
            .fetch_rpc_block_by_hash_with_traces(block_hash, block_number, include_traces)
            .await?;
        self.process_raw_block(raw).await
    }

    /// Process a batch of block numbers in parallel.
    pub async fn process_block_batch<I>(
        &self,
        block_numbers: I,
        options: BlockBatchOptions,
    ) -> Result<Vec<ProcessedBlock>>
    where
        I: IntoIterator<Item = u64>,
    {
        let blocks: Vec<u64> = block_numbers.into_iter().collect();
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        let max_concurrency = options.max_concurrency.max(1);
        let include_traces = options.include_traces;

        let processor = self.clone();
        let processed = stream::iter(blocks.clone())
            .map(move |number| {
                let processor = processor.clone();
                async move {
                    processor
                        .process_block_with_options(number, include_traces)
                        .await
                        .map(|block| (number, block))
                }
            })
            .buffer_unordered(max_concurrency)
            .try_collect::<Vec<_>>()
            .await?;

        let mut ordered = processed;
        ordered.sort_by_key(|(number, _)| *number);
        Ok(ordered.into_iter().map(|(_, block)| block).collect())
    }

    /// Process a previously fetched [`RawBlockData`].
    pub async fn process_raw_block(&self, raw_block: RawBlockData) -> Result<ProcessedBlock> {
        let RawBlockData {
            header,
            transactions,
            receipts,
            traces,
        } = raw_block;

        if receipts.len() != transactions.len() {
            bail!(
                "raw block {} has {} transactions but {} receipts",
                header.number,
                transactions.len(),
                receipts.len()
            );
        }

        if let Some(trace_list) = traces.as_ref() {
            if trace_list.len() != transactions.len() {
                bail!(
                    "raw block {} has {} transactions but {} traces",
                    header.number,
                    transactions.len(),
                    trace_list.len()
                );
            }
        }

        let trace_list = traces.unwrap_or_default();

        let mut processed_transactions = Vec::with_capacity(transactions.len());

        for (index, (metadata, receipt)) in transactions
            .into_iter()
            .zip(receipts.into_iter())
            .enumerate()
        {
            let trace = trace_list.get(index).cloned();
            let processed = self
                .process_single_transaction(&metadata, &receipt, trace.as_ref())
                .await?;

            processed_transactions.push(ProcessedBlockTransactions {
                metadata,
                receipt,
                processed,
                trace,
            });
        }

        Ok(ProcessedBlock {
            header,
            transactions: processed_transactions,
        })
    }

    async fn process_single_transaction(
        &self,
        metadata: &TransactionData,
        receipt: &TransactionReceipt,
        trace: Option<&TransactionTrace>,
    ) -> Result<ProcessedTransaction> {
        let logs = conversion::convert_logs(&receipt.logs);
        let status = receipt.status;

        let access_list: Vec<ProcessedAccessListItem> = metadata
            .access_list
            .iter()
            .map(|item| ProcessedAccessListItem {
                address: item.address,
                storage_keys: item.storage_keys.clone(),
            })
            .collect();
        let blob_versioned_hashes = metadata.blob_versioned_hashes.clone();
        let max_fee_per_blob_gas = metadata.max_fee_per_blob_gas.clone();
        let blob_gas_used = receipt.blob_gas_used;
        let signed_authorizations = metadata.signed_authorizations.clone();

        let mut processed_tx = self
            .tx_processor
            .process_transaction_from_raw_data(
                metadata.hash,
                metadata.block_number,
                metadata.block_timestamp,
                metadata.tx_index,
                metadata.from,
                metadata.to,
                metadata.value,
                metadata.input.clone().to_vec(),
                metadata.gas_price,
                receipt.gas_used,
                status,
                metadata.nonce,
                metadata.transaction_type,
                metadata.max_fee_per_gas.clone(),
                metadata.max_priority_fee_per_gas.clone(),
                logs,
                metadata.gas_limit,
                access_list,
                blob_versioned_hashes,
                max_fee_per_blob_gas,
                blob_gas_used,
                signed_authorizations,
                None,
            )
            .await?;

        let trace_processor = TransactionTraceProcessor::new();
        let internal_transactions = trace
            .map(|trace| {
                let frame = conversion::convert_transaction_trace(trace);
                trace_processor.extract_internal_transactions_from_call_trace(&frame)
            })
            .unwrap_or_default();

        processed_tx.internal_transactions = internal_transactions;
        processed_tx.bribe_amount =
            TxProcessor::calculate_bribe_amount(&processed_tx.internal_transactions);

        let mut balance_calculator = AddressBalanceChangeCalculator::new();
        let balance_changes = balance_calculator.calculate_balance_changes_from_processed_data(
            &processed_tx.erc20_transfers,
            &processed_tx.internal_transactions,
            metadata.block_number,
            metadata.tx_index,
        )?;
        processed_tx.address_balance_changes = balance_changes;

        if metadata.to.is_none() {
            let contract_address = derive_create_address(metadata.from, metadata.nonce);
            processed_tx.contract_address = Some(contract_address);

            if receipt.status {
                processed_tx
                    .contract_creation_events
                    .push(ContractCreationEvent { contract_address });
            }
        }

        Ok(processed_tx)
    }
}

fn derive_create_address(from: Address, nonce: u64) -> Address {
    let mut stream = RlpStream::new_list(2);
    stream.append(&from.as_slice());
    stream.append(&nonce);
    let hash = keccak256(stream.out());
    Address::from_slice(&hash[12..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Bytes, U256};
    use reth_chain_query::provider::{BlockHeader, CallFrame, CallType, TransactionReceipt};

    fn dummy_processor() -> BlockProcessor {
        let rpc_fetcher = RpcBlockDataFetcher::new("http://127.0.0.1:8545").unwrap();
        let fetcher = Arc::new(BlockDataFetcher::rpc_only(rpc_fetcher));
        BlockProcessor::with_block_fetcher(fetcher)
    }

    fn dummy_header() -> BlockHeader {
        BlockHeader {
            number: 1,
            hash: B256::ZERO,
            parent_hash: B256::ZERO,
            timestamp: 12,
            gas_limit: 30_000_000,
            gas_used: 21_000,
            base_fee_per_gas: Some(1),
        }
    }

    fn dummy_transaction() -> TransactionData {
        TransactionData {
            hash: B256::ZERO,
            block_number: 1,
            block_timestamp: 12,
            tx_index: 0,
            tx_number: 0,
            from: Address::ZERO,
            to: Some(Address::ZERO),
            value: U256::ZERO,
            input: Bytes::new(),
            gas_price: U256::from(1),
            gas_limit: 21_000,
            nonce: 0,
            transaction_type: 0,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            access_list: Vec::new(),
            blob_versioned_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            signed_authorizations: Vec::new(),
        }
    }

    fn dummy_receipt() -> TransactionReceipt {
        TransactionReceipt {
            tx_hash: B256::ZERO,
            status: true,
            gas_used: 21_000,
            logs: Vec::new(),
            cumulative_gas_used: 21_000,
            effective_gas_price: U256::from(1),
            contract_address: None,
            blob_gas_used: None,
        }
    }

    fn dummy_trace() -> TransactionTrace {
        TransactionTrace {
            call_frame: CallFrame {
                from: Address::ZERO,
                to: Some(Address::ZERO),
                value: U256::ZERO,
                input: Bytes::new(),
                output: Bytes::new(),
                gas_used: 21_000,
                gas_limit: 21_000,
                depth: 0,
                call_type: CallType::Call,
                subcalls: Vec::new(),
            },
            gas_used: 21_000,
            output: Bytes::new(),
            error: None,
        }
    }

    #[tokio::test]
    async fn process_raw_block_rejects_receipt_count_mismatch() {
        let raw = RawBlockData {
            header: dummy_header(),
            transactions: vec![dummy_transaction()],
            receipts: Vec::new(),
            traces: None,
        };

        let err = dummy_processor()
            .process_raw_block(raw)
            .await
            .expect_err("receipt count mismatch should fail")
            .to_string();

        assert!(err.contains("1 transactions but 0 receipts"));
    }

    #[tokio::test]
    async fn process_raw_block_rejects_trace_count_mismatch() {
        let raw = RawBlockData {
            header: dummy_header(),
            transactions: Vec::new(),
            receipts: Vec::new(),
            traces: Some(vec![dummy_trace()]),
        };

        let err = dummy_processor()
            .process_raw_block(raw)
            .await
            .expect_err("trace count mismatch should fail")
            .to_string();

        assert!(err.contains("0 transactions but 1 traces"));
    }

    #[tokio::test]
    async fn process_raw_block_accepts_missing_traces() {
        let raw = RawBlockData {
            header: dummy_header(),
            transactions: Vec::new(),
            receipts: Vec::new(),
            traces: None,
        };

        let processed = dummy_processor()
            .process_raw_block(raw)
            .await
            .expect("empty block without traces should process");

        assert!(processed.transactions.is_empty());
    }

    #[tokio::test]
    async fn process_raw_block_accepts_matching_empty_trace_list() {
        let raw = RawBlockData {
            header: dummy_header(),
            transactions: Vec::new(),
            receipts: Vec::new(),
            traces: Some(Vec::new()),
        };

        let processed = dummy_processor()
            .process_raw_block(raw)
            .await
            .expect("empty block with empty traces should process");

        assert!(processed.transactions.is_empty());
    }

    #[test]
    fn dummy_receipt_shape_is_valid() {
        assert_eq!(dummy_receipt().tx_hash, B256::ZERO);
    }
}
