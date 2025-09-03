/// Block-level transaction operations
/// 
/// Efficiently loads all transactions in a block with:
/// - Transaction data from local DB (Transactions table)
/// - Receipts and logs from local DB (Receipts table)
/// - Block metadata from local DB (Headers table)
/// - Traces from RPC (debug_traceBlockByNumber) - until we have local tracing

use alloy_primitives::{Address, B256, U256, Bytes};
use alloy_consensus::transaction::SignerRecoverable;
use alloy_consensus::Transaction;
use eyre::Result;
use futures::stream::{self, Stream, StreamExt, TryStreamExt};
use reth_provider::{BlockNumReader, HeaderProvider, BlockReader, ReceiptProvider, TransactionsProvider, BlockBodyIndicesProvider};
use std::time::Instant;

use super::{
    RethQueryProvider, TransactionData, TransactionReceipt, TransactionTrace,
    BlockTransactions, FullTransactionData,
    BlockTransactionOptions, Log,
    StateChanges, CallFrame, CallType
};
use super::types::{TransactionMetadata, BlockHeader};

impl RethQueryProvider {
    /// Get transaction indices for a block (first_tx_num and count)
    pub fn get_block_tx_indices(&self, block_number: u64) -> Result<reth_db_models::StoredBlockBodyIndices> {
        let provider = self.provider_factory.provider()?;
        provider.block_body_indices(block_number)?
            .ok_or_else(|| eyre::eyre!("No transaction indices for block {}", block_number))
    }
    
    /// Get block header from Headers table
    pub async fn get_block_header(&self, block_number: u64) -> Result<BlockHeader> {
        let provider = self.provider_factory.provider()?;
        
        let header = provider.header_by_number(block_number)?
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
    pub async fn get_block_transactions(
        &self,
        block_number: u64,
        options: BlockTransactionOptions,
    ) -> Result<BlockTransactions> {
        let start_time = Instant::now();
        
        // 1. Get block header from DB
        let header = self.get_block_header(block_number).await?;
        
        // 2. Get all transactions in block from DB
        let transactions = self.get_block_transactions_from_db(block_number)?;
        
        // 3. Get all receipts from DB (includes logs)
        let receipts = self.get_block_receipts_from_db(block_number)?;
        
        // 4. Optional: Get traces via RPC if requested
        let traces = if options.include_traces {
            Some(self.get_block_traces(block_number).await?)
        } else {
            None
        };
        
        // 5. Combine all data into FullTransactionData objects
        let mut complete_txs = Vec::new();
        for (idx, (tx_metadata, receipt)) in transactions.into_iter().zip(receipts).enumerate() {
            // Get trace for this transaction if available
            let tx_trace = traces.as_ref().and_then(|t| t.get(idx).cloned());
            
            // Calculate state changes if requested
            let state_changes = if options.include_state_changes && tx_trace.is_some() {
                Some(self.calculate_state_changes(&tx_trace.unwrap())?)
            } else {
                None
            };
            
            complete_txs.push(FullTransactionData {
                tx_metadata,
                tx_receipt: receipt,
                tx_trace,
                state_changes,
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
    pub async fn get_block_tx_metadata(&self, block_number: u64) -> Result<Vec<TransactionMetadata>> {
        self.get_block_transactions_from_db(block_number)
    }
    
    /// Stream block transactions for large blocks
    /// Memory-efficient processing for blocks with many transactions
    pub fn stream_block_transactions(
        &self,
        block_number: u64,
        batch_size: usize,
    ) -> impl Stream<Item = Result<Vec<FullTransactionData>>> + '_ {
        stream::try_unfold(0usize, move |offset| async move {
            let transactions = self.get_block_transactions_batch(block_number, offset, batch_size).await?;
            
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
        
        let block_txs = self.get_block_transactions(block_number, options).await?;
        
        Ok(block_txs.transactions
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect())
    }
    
    /// FROM DATABASE - Get all transactions in a block
    fn get_block_transactions_from_db(&self, block_number: u64) -> Result<Vec<TransactionMetadata>> {
        let provider = self.provider_factory.provider()?;
        
        // Get block body indices to find transaction range
        let indices = self.get_block_tx_indices(block_number)?
            .ok_or_else(|| eyre::eyre!("No body indices for block {}", block_number))?;
        
        // Get block header for timestamp
        let header = provider.header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block {}", block_number))?;
        
        // Batch fetch all transactions in the block using tx range
        let tx_range = indices.first_tx_num..indices.first_tx_num + indices.tx_count;
        let txs = provider.transactions_by_tx_range(tx_range.clone())?;
        
        // Build transaction metadata for each transaction
        let mut transactions = Vec::new();
        for (idx, tx) in txs.into_iter().enumerate() {
            let from = tx.recover_signer()
                .map_err(|_| eyre::eyre!("Failed to recover signer"))?;
            
            let tx_num = indices.first_tx_num + idx as u64;
            
            transactions.push(TransactionMetadata {
                hash: tx.hash(),
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
    
    /// FROM DATABASE - Get all receipts for block
    fn get_block_receipts_from_db(&self, block_number: u64) -> Result<Vec<TransactionReceipt>> {
        let provider = self.provider_factory.provider()?;
        
        // Get receipts by block
        let receipts = provider.receipts_by_block(block_number.into())?
            .ok_or_else(|| eyre::eyre!("No receipts for block {}", block_number))?;
        
        // Get block header and indices for transaction info
        let indices = self.get_block_tx_indices(block_number)?
            .ok_or_else(|| eyre::eyre!("No body indices for block"))?;
        
        let header = provider.header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block"))?;
        
        // Batch fetch all transactions for hash and effective gas price
        let tx_range = indices.first_tx_num..indices.first_tx_num + indices.tx_count;
        let txs = provider.transactions_by_tx_range(tx_range)?;
        
        // Zip transactions with receipts to build the result
        let mut tx_receipts = Vec::new();
        for (idx, (tx, receipt)) in txs.into_iter().zip(receipts).enumerate() {
            // Convert logs
            let logs = receipt.logs.iter().enumerate().map(|(log_idx, log)| {
                Log {
                    address: log.address,
                    topics: log.topics().to_vec(),
                    data: log.data.data.clone(),
                    log_index: log_idx as u64,
                    transaction_index: idx as u64,
                    block_number,
                }
            }).collect();
            
            tx_receipts.push(TransactionReceipt {
                tx_hash: tx.hash(),
                status: receipt.success,
                gas_used: receipt.cumulative_gas_used,
                logs,
                cumulative_gas_used: receipt.cumulative_gas_used,
                effective_gas_price: U256::from(tx.effective_gas_price(header.base_fee_per_gas)),
            });
        }
        
        Ok(tx_receipts)
    }
    
    /// Get traces for all transactions in a block
    async fn get_block_traces(&self, block_number: u64) -> Result<Vec<TransactionTrace>> {
        // Try RPC first if available
        if let Some(_rpc_provider) = &self.rpc_provider {
            match self.get_block_traces_from_rpc(block_number).await {
                Ok(traces) => return Ok(traces),
                Err(e) => {
                    eprintln!("Failed to get traces from RPC, falling back to simulation: {}", e);
                }
            }
        }
        
        // Fallback to local simulation
        self.get_block_traces_local(block_number).await
    }
    
    /// FROM RPC - Get traces for entire block
    async fn get_block_traces_from_rpc(&self, block_number: u64) -> Result<Vec<TransactionTrace>> {
        let _rpc_provider = self.rpc_provider.as_ref()
            .ok_or_else(|| eyre::eyre!("RPC provider not configured"))?;
        
        // Use debug_traceBlockByNumber
        // Note: Implementation depends on alloy RPC support
        
        Err(eyre::eyre!("RPC block traces not yet implemented"))
    }
    
    /// LOCAL SIMULATION - Simulate all transactions in block
    async fn get_block_traces_local(&self, block_number: u64) -> Result<Vec<TransactionTrace>> {
        let transactions = self.get_block_transactions_from_db(block_number)?;
        let mut traces = Vec::new();
        
        for tx in transactions {
            // Build call request from transaction
            let call_request = tx_simulator::UnsignedTransaction {
                from: Some(tx.from),
                to: tx.to,
                value: Some(tx.value),
                data: if tx.input.is_empty() { None } else { Some(tx.input.into()) },
                gas: Some(tx.gas_limit),
                ..Default::default()
            };
            
            // Simulate at block_number - 1
            let simulation_block = block_number.saturating_sub(1);
            
            match self.tx_simulator.simulate_unsigned_transaction_with_full_trace_at_block(call_request, simulation_block).await {
                Ok(result) => {
                    traces.push(TransactionTrace {
                        call_frame: self.convert_call_frame(&result.call_trace),
                        gas_used: result.gas_used,
                        output: result.call_trace.output.clone().unwrap_or_default(),
                        error: if !result.success {
                            result.revert_reason
                        } else {
                            None
                        },
                    });
                }
                Err(e) => {
                    // Create error trace
                    traces.push(TransactionTrace {
                        call_frame: CallFrame {
                            from: tx.from,
                            to: tx.to,
                            value: tx.value,
                            input: tx.input,
                            output: Bytes::new(),
                            gas_used: 0,
                            gas_limit: tx.gas_limit,
                            depth: 0,
                            call_type: CallType::Call,
                            subcalls: Vec::new(),
                        },
                        gas_used: 0,
                        output: Bytes::new(),
                        error: Some(format!("Simulation failed: {}", e)),
                    });
                }
            }
        }
        
        Ok(traces)
    }
    
    /// Extract internal transactions from call frame
    /// Calculate state changes from trace
    fn calculate_state_changes(&self, _trace: &TransactionTrace) -> Result<StateChanges> {
        // This would require more complex trace analysis
        // For now, return empty state changes
        Ok(StateChanges {
            balance_changes: Vec::new(),
            storage_changes: Vec::new(),
            nonce_changes: Vec::new(),
        })
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
            .map(|block| self.get_block_transactions(block, options.clone()))
            .buffer_unordered(4) // Process 4 blocks in parallel
            .try_collect()
            .await?;
        
        Ok(results)
    }
    
    /// Get block transaction count without loading all data
    pub async fn get_block_transaction_count(&self, block_number: u64) -> Result<usize> {
        let provider = self.provider_factory.provider()?;
        let indices = self.get_block_tx_indices(block_number)?
            .ok_or_else(|| eyre::eyre!("No body indices for block {}", block_number))?;
        Ok(indices.tx_count as usize)
    }
    
    /// Check if block needs trace data (has contract interactions)
    pub async fn block_needs_traces(&self, block_number: u64) -> Result<bool> {
        let transactions = self.get_block_tx_metadata(block_number).await?;
        
        // Check if any transaction has input data (contract interaction)
        Ok(transactions.iter().any(|tx| !tx.input.is_empty() && tx.to.is_some()))
    }
}