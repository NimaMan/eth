use alloy_consensus::transaction::SignerRecoverable;
use alloy_consensus::Transaction;
/// Block-level transaction operations
///
/// Efficiently loads all transactions in a block with:
/// - Transaction data from local DB (Transactions table)
/// - Receipts and logs from local DB (Receipts table)
/// - Block metadata from local DB (Headers table)
/// - Traces from local simulation (matches debug_traceBlockByNumber)
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_rpc_types_trace::geth::{GethDebugTracingOptions, GethTrace, TraceResult};
use eyre::Result;
use futures::stream::{self, Stream, StreamExt, TryStreamExt};
use reth_primitives::{Recovered, TransactionSignedEcRecovered};
use reth_provider::{
    BlockBodyIndicesProvider, BlockNumReader, BlockReader, HeaderProvider, ReceiptProvider,
    TransactionsProvider,
};
use std::time::Instant;
use tx_simulator::block_simulation::BlockTracer;

use super::types::{Block, BlockHeader, RawBlockData, TransactionMetadata};
use super::{
    BlockTransactionOptions, BlockTransactions, CallFrame, CallType, FullTransactionData, Log,
    RethQueryProvider, TransactionData, TransactionReceipt, TransactionTrace,
};

impl RethQueryProvider {
    /// Get transaction indices for a block (first_tx_num and count)
    pub fn get_block_tx_indices(
        &self,
        block_number: u64,
    ) -> Result<reth_db_models::StoredBlockBodyIndices> {
        let provider = self.provider_factory.provider()?;
        provider
            .block_body_indices(block_number)?
            .ok_or_else(|| eyre::eyre!("No transaction indices for block {}", block_number))
    }

    /// Get block header from Headers table
    pub async fn fetch_block_header_only(&self, block_number: u64) -> Result<BlockHeader> {
        let provider = self.provider_factory.provider()?;

        let header = provider
            .header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block {}", block_number))?;

        Ok(BlockHeader {
            number: block_number,
            hash: header.hash_slow(),
            parent_hash: header.parent_hash,
            timestamp: header.timestamp,
            gas_limit: header.gas_limit,
            gas_used: header.gas_used,
            base_fee_per_gas: header.base_fee_per_gas,
        })
    }

    /// Get all transactions in a block with complete data
    /// This is the main entry point for block-level processing
    pub async fn fetch_block_with_all_tx_data(
        &self,
        block_number: u64,
        options: BlockTransactionOptions,
    ) -> Result<BlockTransactions> {
        let start_time = Instant::now();

        let RawBlockData {
            header,
            transactions,
            receipts,
            traces,
        } = self
            .fetch_raw_block_data(block_number, options.include_traces)
            .await?;

        let traces = traces.unwrap_or_default();

        let mut complete_txs = Vec::new();
        for (idx, (tx_metadata, receipt)) in transactions.into_iter().zip(receipts).enumerate() {
            // Get trace for this transaction if available
            let tx_trace = traces.get(idx).cloned();

            complete_txs.push(FullTransactionData {
                tx_metadata,
                tx_receipt: receipt,
                tx_trace,
                state_changes: None,
            });
        }

        Ok(BlockTransactions {
            block_number,
            block_hash: header.hash,
            timestamp: header.timestamp,
            gas_used: header.gas_used,
            gas_limit: header.gas_limit,
            base_fee_per_gas: header.base_fee_per_gas,
            transactions: complete_txs,
        })
    }

    /// Get only transaction metadata without receipts, logs, or traces
    /// Returns just the transaction parameters (from, to, value, input, gas, nonce)
    /// Use this when you don't need execution results, events, or traces
    pub async fn fetch_block_tx_metadata_only(
        &self,
        block_number: u64,
    ) -> Result<Vec<TransactionMetadata>> {
        self.fetch_block_tx_metadata_only_internal(block_number)
    }

    /// Get only transaction hashes for a block using efficient index lookup
    /// This is the fastest way to get all transaction hashes in a block
    pub async fn fetch_block_tx_hashes_using_indices(
        &self,
        block_number: u64,
    ) -> Result<Vec<B256>> {
        let provider = self.provider_factory.provider()?;

        // Get block indices to find transaction range
        let indices = self.get_block_tx_indices(block_number)?;

        // Batch fetch all transactions in range just for hashes
        let tx_range = indices.first_tx_num..indices.first_tx_num + indices.tx_count;
        let txs = provider.transactions_by_tx_range(tx_range)?;

        // Extract just the hashes
        Ok(txs.iter().map(|tx| *tx.hash()).collect())
    }

    /// Get only receipts for all transactions in a block
    /// Public wrapper for the internal method
    pub async fn fetch_block_receipts_only(
        &self,
        block_number: u64,
    ) -> Result<Vec<TransactionReceipt>> {
        self.fetch_block_receipts_only_internal(block_number)
    }

    /// Get both transaction metadata and receipts (common use case)
    /// Returns paired metadata and receipts for each transaction
    pub async fn fetch_block_tx_metadata_and_receipts(
        &self,
        block_number: u64,
    ) -> Result<Vec<(TransactionMetadata, TransactionReceipt)>> {
        let metadata = self.fetch_block_tx_metadata_only_internal(block_number)?;
        let receipts = self.fetch_block_receipts_only_internal(block_number)?;

        // Zip them together
        Ok(metadata.into_iter().zip(receipts).collect())
    }

    /// Stream block transactions for large blocks
    /// Memory-efficient processing for blocks with many transactions
    pub fn stream_block_transactions(
        &self,
        block_number: u64,
        batch_size: usize,
    ) -> impl Stream<Item = Result<Vec<FullTransactionData>>> + '_ {
        stream::try_unfold(0usize, move |offset| async move {
            let transactions = self
                .get_block_transactions_batch(block_number, offset, batch_size)
                .await?;

            if transactions.is_empty() {
                Ok(None)
            } else {
                let next_offset = offset + transactions.len();
                Ok(Some((transactions, next_offset)))
            }
        })
    }

    /// Get a batch of transactions from a block
    async fn get_block_transactions_batch(
        &self,
        block_number: u64,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<FullTransactionData>> {
        let options = BlockTransactionOptions {
            include_traces: true,
            include_state_changes: true,
        };

        let block_txs = self
            .fetch_block_with_all_tx_data(block_number, options)
            .await?;

        Ok(block_txs
            .transactions
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect())
    }

    /// FROM DATABASE - Get all transactions in a block (internal method)
    fn fetch_block_tx_metadata_only_internal(
        &self,
        block_number: u64,
    ) -> Result<Vec<TransactionMetadata>> {
        let provider = self.provider_factory.provider()?;

        // Get block body indices to find transaction range
        let indices = self.get_block_tx_indices(block_number)?;

        // Get block header for timestamp
        let header = provider
            .header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block {}", block_number))?;

        // Batch fetch all transactions in the block using tx range
        let tx_range = indices.first_tx_num..indices.first_tx_num + indices.tx_count;
        let txs = provider.transactions_by_tx_range(tx_range.clone())?;

        // Build transaction metadata for each transaction
        let mut transactions = Vec::new();
        for (idx, tx) in txs.into_iter().enumerate() {
            let from = tx
                .recover_signer()
                .map_err(|_| eyre::eyre!("Failed to recover signer"))?;

            let tx_num = indices.first_tx_num + idx as u64;

            transactions.push(TransactionMetadata {
                hash: *tx.hash(),
                block_number,
                block_timestamp: header.timestamp,
                tx_index: idx as u64,
                tx_number: tx_num,
                from,
                to: tx.to(),
                value: tx.value(),
                input: Bytes::from(tx.input().to_vec()),
                gas_price: U256::from(tx.max_fee_per_gas()),
                gas_limit: tx.gas_limit(),
                nonce: tx.nonce(),
                transaction_type: tx.tx_type() as u8,
            });
        }

        Ok(transactions)
    }

    /// FROM DATABASE - Get all receipts for block (internal method)
    fn fetch_block_receipts_only_internal(
        &self,
        block_number: u64,
    ) -> Result<Vec<TransactionReceipt>> {
        let provider = self.provider_factory.provider()?;

        // Get receipts by block
        let receipts = provider
            .receipts_by_block(block_number.into())?
            .ok_or_else(|| eyre::eyre!("No receipts for block {}", block_number))?;

        // Get block header and indices for transaction info
        let indices = self.get_block_tx_indices(block_number)?;

        let header = provider
            .header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block"))?;

        // Batch fetch all transactions for hash and effective gas price
        let tx_range = indices.first_tx_num..indices.first_tx_num + indices.tx_count;
        let txs = provider.transactions_by_tx_range(tx_range)?;

        // Zip transactions with receipts to build the result
        let mut tx_receipts = Vec::new();
        let mut previous_cumulative = 0u64;

        for (idx, (tx, receipt)) in txs.into_iter().zip(receipts).enumerate() {
            // Convert logs
            let logs = receipt
                .logs
                .iter()
                .enumerate()
                .map(|(log_idx, log)| Log {
                    address: log.address,
                    topics: log.topics().to_vec(),
                    data: log.data.data.clone(),
                    log_index: log_idx as u64,
                    transaction_index: idx as u64,
                    block_number,
                })
                .collect();

            let cumulative = receipt.cumulative_gas_used;
            let gas_used = if idx == 0 {
                cumulative
            } else {
                cumulative.saturating_sub(previous_cumulative)
            };
            previous_cumulative = cumulative;

            tx_receipts.push(TransactionReceipt {
                tx_hash: *tx.hash(),
                status: receipt.success,
                gas_used,
                logs,
                cumulative_gas_used: cumulative,
                effective_gas_price: U256::from(tx.effective_gas_price(header.base_fee_per_gas)),
                contract_address: None,
            });
        }

        Ok(tx_receipts)
    }

    /// Fetch raw block data (metadata, transactions, receipts, and optional traces)
    pub async fn fetch_raw_block_data(
        &self,
        block_number: u64,
        include_traces: bool,
    ) -> Result<RawBlockData> {
        let header = self.fetch_block_header_only(block_number).await?;
        let transactions = self.fetch_block_tx_metadata_only_internal(block_number)?;
        let receipts = self.fetch_block_receipts_only_internal(block_number)?;

        let traces = if include_traces {
            Some(self.get_block_traces(block_number).await?)
        } else {
            None
        };

        Ok(RawBlockData {
            header,
            transactions,
            receipts,
            traces,
        })
    }

    /// Get traces for all transactions in a block
    async fn get_block_traces(&self, block_number: u64) -> Result<Vec<TransactionTrace>> {
        // Try RPC first if available
        if let Some(_rpc_provider) = &self.rpc_provider {
            match self.get_block_traces_from_rpc(block_number).await {
                Ok(traces) => return Ok(traces),
                Err(e) => {
                    eprintln!(
                        "Failed to get traces from RPC, falling back to simulation: {}",
                        e
                    );
                }
            }
        }

        // Fallback to simulator-based tracing
        self.simulate_block_traces(block_number).await
    }

    /// FROM RPC - Get traces for entire block
    async fn get_block_traces_from_rpc(&self, block_number: u64) -> Result<Vec<TransactionTrace>> {
        let _rpc_provider = self
            .rpc_provider
            .as_ref()
            .ok_or_else(|| eyre::eyre!("RPC provider not configured"))?;

        // Use debug_traceBlockByNumber
        // Note: Implementation depends on alloy RPC support

        Err(eyre::eyre!("RPC block traces not yet implemented"))
    }

    /// Simulate all transactions in the block to produce call traces
    async fn simulate_block_traces(&self, block_number: u64) -> Result<Vec<TransactionTrace>> {
        let tracer = BlockTracer::new(&self.tx_simulator);
        let trace_results = tracer
            .trace_block_by_number(block_number, Some(GethDebugTracingOptions::default()))
            .await?;

        let mut traces = Vec::with_capacity(trace_results.len());

        for trace in trace_results {
            match trace {
                TraceResult::Success { result, .. } => match result {
                    GethTrace::CallTracer(frame) => {
                        let gas_used: u64 = frame.gas_used.try_into().unwrap_or(u64::MAX);
                        let output = frame.output.clone().unwrap_or_else(Bytes::new);
                        let error = frame.error.clone().or(frame.revert_reason.clone());

                        traces.push(TransactionTrace {
                            call_frame: self.convert_call_frame(&frame),
                            gas_used,
                            output,
                            error,
                        });
                    }
                    other => {
                        return Err(eyre::eyre!(
                            "Unsupported trace variant returned: {:?}",
                            other
                        ));
                    }
                },
                TraceResult::Error { error, .. } => {
                    return Err(eyre::eyre!("Trace error: {:?}", error));
                }
            }
        }

        Ok(traces)
    }

    /// Process multiple blocks in parallel
    pub async fn get_blocks_transactions(
        &self,
        start_block: u64,
        end_block: u64,
        options: BlockTransactionOptions,
    ) -> Result<Vec<BlockTransactions>> {
        let blocks = (start_block..=end_block).collect::<Vec<_>>();

        let results = stream::iter(blocks)
            .map(|block| self.fetch_block_with_all_tx_data(block, options.clone()))
            .buffer_unordered(4) // Process 4 blocks in parallel
            .try_collect()
            .await?;

        Ok(results)
    }

    /// Get block transaction count without loading all data
    pub async fn get_block_transaction_count(&self, block_number: u64) -> Result<usize> {
        let provider = self.provider_factory.provider()?;
        let indices = self.get_block_tx_indices(block_number)?;
        Ok(indices.tx_count as usize)
    }

    /// Check if block needs trace data (has contract interactions)
    pub async fn block_needs_traces(&self, block_number: u64) -> Result<bool> {
        let transactions = self.fetch_block_tx_metadata_only(block_number).await?;

        // Check if any transaction has input data (contract interaction)
        Ok(transactions
            .iter()
            .any(|tx| !tx.input.is_empty() && tx.to.is_some()))
    }

    // === Legacy helper methods kept for downstream crates ===

    /// Get block with transactions (compatibility method for examples)
    /// Returns block header and raw transaction list
    pub async fn get_block_with_txs(&self, block_number: u64) -> Result<Block> {
        let provider = self.provider_factory.provider()?;

        // Get header
        let header = self.fetch_block_header_only(block_number).await?;

        // Get block indices to find transaction range
        let indices = self.get_block_tx_indices(block_number)?;

        // Batch fetch all transactions
        let tx_range = indices.first_tx_num..indices.first_tx_num + indices.tx_count;
        let txs = provider.transactions_by_tx_range(tx_range)?;

        // Convert to TransactionSignedEcRecovered (with recovered senders)
        let transactions: Result<Vec<_>> = txs
            .into_iter()
            .map(|tx| {
                let sender = tx
                    .recover_signer()
                    .map_err(|_| eyre::eyre!("Failed to recover signer"))?;
                Ok(Recovered::new_unchecked(tx, sender))
            })
            .collect();

        Ok(Block {
            header,
            transactions: transactions?,
        })
    }

    /// Get all block transactions with default options (compatibility wrapper)
    pub async fn get_block_transactions(&self, block_number: u64) -> Result<BlockTransactions> {
        self.fetch_block_with_all_tx_data(block_number, BlockTransactionOptions::default())
            .await
    }

    /// Get block receipts (compatibility alias)
    pub async fn get_block_receipts(&self, block_number: u64) -> Result<Vec<TransactionReceipt>> {
        self.fetch_block_receipts_only(block_number).await
    }

    /// Get block transactions with options (compatibility alias)
    pub async fn get_block_transactions_with_options(
        &self,
        block_number: u64,
        options: BlockTransactionOptions,
    ) -> Result<BlockTransactions> {
        self.fetch_block_with_all_tx_data(block_number, options)
            .await
    }
}
