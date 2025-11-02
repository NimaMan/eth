mod conversion;
pub mod types;

use crate::tx_processor::data_models::{
    ContractCreationEvent, ProcessedAccessListItem, ProcessedTransaction,
};
use crate::tx_processor::{AddressBalanceChangeCalculator, TransactionTraceProcessor, TxProcessor};
use alloy_primitives::{keccak256, Address};
use eyre::Result;
use futures::stream::{self, StreamExt, TryStreamExt};
use reth_chain_query::{
    provider::{RawBlockData, TransactionData, TransactionReceipt, TransactionTrace},
    RethQueryProvider,
};
use rlp::RlpStream;
use std::sync::Arc;

pub use types::{BlockBatchOptions, ProcessedBlock, ProcessedBlockTransaction};

/// High-level orchestration for processing entire blocks worth of transactions.
#[derive(Clone)]
pub struct BlockProcessor {
    provider: Arc<RethQueryProvider>,
    tx_processor: Arc<TxProcessor>,
}

impl BlockProcessor {
    /// Build a block processor with a shared provider handle.
    pub fn new(provider: Arc<RethQueryProvider>) -> Self {
        Self {
            provider,
            tx_processor: Arc::new(TxProcessor::new()),
        }
    }

    /// Build a block processor re-using an existing [`TxProcessor`].
    pub fn with_tx_processor(
        provider: Arc<RethQueryProvider>,
        tx_processor: Arc<TxProcessor>,
    ) -> Self {
        Self {
            provider,
            tx_processor,
        }
    }

    /// Access the underlying query provider.
    pub fn provider(&self) -> &Arc<RethQueryProvider> {
        &self.provider
    }

    /// Access the transaction processor used for decoding.
    pub fn tx_processor(&self) -> &Arc<TxProcessor> {
        &self.tx_processor
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
            .provider
            .fetch_raw_block_data(block_number, include_traces)
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

            processed_transactions.push(ProcessedBlockTransaction {
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
                None,
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
