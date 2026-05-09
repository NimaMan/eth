use alloy_consensus::transaction::{SignerRecoverable, TransactionMeta, TxType};
use alloy_consensus::Transaction as _;
/// Transaction data access methods
///
/// This module provides all transaction-related queries that tx_processor needs:
/// - Loading transactions by hash or number
/// - Getting transaction receipts and logs
/// - Building CallRequests for simulation
/// - Accessing transaction metadata
use alloy_eips::eip4844::DATA_GAS_PER_BLOB;
use alloy_primitives::{Address, Bytes, B256, U256};
use eyre::Result;
use reth_provider::{HeaderProvider, ReceiptProvider, TransactionsProvider};
use tx_simulator::UnsignedTransaction;

use super::{
    CallFrame, CallType, Log, RethQueryProvider, TransactionData, TransactionReceipt,
    TransactionTrace, TransactionWithTrace,
};

impl RethQueryProvider {
    fn extract_dynamic_fee_fields(
        tx: &reth_ethereum_primitives::TransactionSigned,
    ) -> (Option<U256>, Option<U256>) {
        let tx_type = tx.tx_type();
        let max_fee = tx.max_fee_per_gas();
        let max_priority = tx.max_priority_fee_per_gas();
        match tx_type {
            TxType::Eip1559 | TxType::Eip4844 | TxType::Eip7702 => {
                (Some(U256::from(max_fee)), max_priority.map(U256::from))
            }
            _ => (None, None),
        }
    }
    // === Private Helper Methods ===

    /// Private helper to get raw transaction by number
    fn get_raw_tx_by_number(
        &self,
        tx_number: u64,
    ) -> Result<reth_ethereum_primitives::TransactionSigned> {
        let provider = self.provider_factory.provider()?;
        provider
            .transaction_by_id(tx_number)?
            .ok_or_else(|| eyre::eyre!("Transaction number {} not found", tx_number))
    }

    /// Private helper to get transaction with metadata
    fn get_raw_tx_with_metadata(
        &self,
        tx_hash: B256,
    ) -> Result<(reth_ethereum_primitives::TransactionSigned, TransactionMeta)> {
        let provider = self.provider_factory.provider()?;
        provider
            .transaction_by_hash_with_meta(tx_hash)?
            .ok_or_else(|| eyre::eyre!("Transaction {} not found", tx_hash))
    }

    // === Public Methods ===

    /// Get complete transaction data by hash
    /// Used by tx_processor to load transaction for processing
    pub async fn get_transaction_by_hash(&self, tx_hash: B256) -> Result<TransactionData> {
        // Get transaction with metadata
        let (tx, meta) = self.get_raw_tx_with_metadata(tx_hash)?;

        let block_number = meta.block_number;
        let tx_index = meta.index as u64;

        let provider = self.provider_factory.provider()?;
        // Get block timestamp from Headers table
        let header = provider
            .header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("Header not found for block {}", block_number))?;

        // Calculate tx_number from block body indices
        let indices = self.get_block_tx_indices(block_number)?;
        let tx_number = indices.first_tx_num + tx_index;

        // Extract transaction details
        let from = tx
            .recover_signer()
            .map_err(|_| eyre::eyre!("Failed to recover signer for {}", tx_hash))?;

        let (max_fee_per_gas, max_priority_fee_per_gas) = Self::extract_dynamic_fee_fields(&tx);

        let access_list = tx
            .access_list()
            .map(|list| list.to_vec())
            .unwrap_or_default();
        let blob_versioned_hashes = tx
            .blob_versioned_hashes()
            .map(|hashes| hashes.to_vec())
            .unwrap_or_default();
        let max_fee_per_blob_gas = tx.max_fee_per_blob_gas().map(U256::from);
        let signed_authorizations = tx
            .authorization_list()
            .map(|auth| auth.to_vec())
            .unwrap_or_default();

        Ok(TransactionData {
            hash: tx_hash,
            block_number,
            block_timestamp: header.timestamp,
            tx_index,
            tx_number,
            from,
            to: tx.to(),
            value: tx.value(),
            input: Bytes::from(tx.input().to_vec()),
            gas_price: U256::from(tx.max_fee_per_gas()),
            gas_limit: tx.gas_limit(),
            nonce: tx.nonce(),
            transaction_type: tx.tx_type() as u8,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            access_list,
            blob_versioned_hashes,
            max_fee_per_blob_gas,
            signed_authorizations,
        })
    }

    /// Get transaction by sequential number (Txumber)
    /// More efficient than by hash as it's the primary key
    pub async fn get_transaction_by_number(&self, tx_number: u64) -> Result<TransactionData> {
        // Get transaction directly by ID
        let tx = self.get_raw_tx_by_number(tx_number)?;

        let tx_hash = tx.hash();

        // We need to find which block this transaction is in
        // This is less efficient than by hash, but still works
        // In practice, you'd want to maintain an index for this
        let (tx, meta) = self.get_raw_tx_with_metadata(*tx_hash)?;

        let block_number = meta.block_number;
        let tx_index = meta.index as u64;

        let provider = self.provider_factory.provider()?;
        // Get block header for timestamp
        let header = provider
            .header_by_number(block_number)?
            .ok_or_else(|| eyre::eyre!("Header not found"))?;

        let from = tx
            .recover_signer()
            .map_err(|_| eyre::eyre!("Failed to recover signer"))?;

        let (max_fee_per_gas, max_priority_fee_per_gas) = Self::extract_dynamic_fee_fields(&tx);

        let access_list = tx
            .access_list()
            .map(|list| list.to_vec())
            .unwrap_or_default();
        let blob_versioned_hashes = tx
            .blob_versioned_hashes()
            .map(|hashes| hashes.to_vec())
            .unwrap_or_default();
        let max_fee_per_blob_gas = tx.max_fee_per_blob_gas().map(U256::from);
        let signed_authorizations = tx
            .authorization_list()
            .map(|auth| auth.to_vec())
            .unwrap_or_default();

        Ok(TransactionData {
            hash: *tx_hash,
            block_number,
            block_timestamp: header.timestamp,
            tx_index,
            tx_number,
            from,
            to: tx.to(),
            value: tx.value(),
            input: Bytes::from(tx.input().to_vec()),
            gas_price: U256::from(tx.max_fee_per_gas()),
            gas_limit: tx.gas_limit(),
            nonce: tx.nonce(),
            transaction_type: tx.tx_type() as u8,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            access_list,
            blob_versioned_hashes,
            max_fee_per_blob_gas,
            signed_authorizations,
        })
    }

    /// Get transaction receipt including logs
    /// Returns status, gas used, and all event logs
    pub async fn get_transaction_receipt(&self, tx_hash: B256) -> Result<TransactionReceipt> {
        let provider = self.provider_factory.provider()?;

        // Get receipt from Receipts table
        let receipt = provider
            .receipt_by_hash(tx_hash)?
            .ok_or_else(|| eyre::eyre!("Receipt not found for {}", tx_hash))?;

        // Get transaction metadata for block info
        let (tx, meta) = self.get_raw_tx_with_metadata(tx_hash)?;

        // Get header for base fee
        let header = provider
            .header_by_number(meta.block_number)?
            .ok_or_else(|| eyre::eyre!("Header not found"))?;

        // Convert logs
        let logs = receipt
            .logs
            .iter()
            .enumerate()
            .map(|(idx, log)| Log {
                address: log.address,
                topics: log.topics().to_vec(),
                data: log.data.data.clone(),
                log_index: idx as u64,
                transaction_index: meta.index as u64,
                block_number: meta.block_number,
            })
            .collect();

        let blob_gas_used = tx
            .blob_versioned_hashes()
            .map(|hashes| hashes.len() as u64 * DATA_GAS_PER_BLOB);

        Ok(TransactionReceipt {
            tx_hash,
            status: receipt.success,
            gas_used: receipt.cumulative_gas_used,
            logs,
            cumulative_gas_used: receipt.cumulative_gas_used,
            effective_gas_price: U256::from(tx.effective_gas_price(header.base_fee_per_gas)),
            contract_address: None,
            blob_gas_used,
        })
    }

    /// Get logs for a specific transaction
    /// Returns all event logs emitted by the transaction
    pub async fn get_transaction_logs(&self, tx_hash: B256) -> Result<Vec<Log>> {
        let receipt = self.get_transaction_receipt(tx_hash).await?;
        Ok(receipt.logs)
    }

    /// Get mempool arrival timestamp (milliseconds since epoch) if recorded
    pub fn get_tx_arrival_ms(&self, tx_hash: B256) -> Result<Option<u64>> {
        let provider = self.provider_factory.provider()?;
        let Some(tx_id) = provider.transaction_id(tx_hash)? else {
            return Ok(None);
        };

        let Some(db) = self.reth_index() else {
            return Ok(None);
        };

        db.get_tx_arrival_ms(tx_id as u64)
    }

    /// Build UnsignedTransaction from transaction hash for re-simulation
    /// Loads on-chain transaction and converts it to UnsignedTransaction format
    /// Used by tx_processor to re-simulate historical transactions for trace extraction
    pub async fn build_unsigned_tx_from_tx_hash(
        &self,
        tx_hash: B256,
    ) -> Result<UnsignedTransaction> {
        let tx_data = self.get_transaction_by_hash(tx_hash).await?;

        Ok(UnsignedTransaction {
            from: Some(tx_data.from),
            to: tx_data.to,
            value: Some(tx_data.value),
            data: if tx_data.input.is_empty() {
                None
            } else {
                Some(tx_data.input.clone())
            },
            gas: Some(tx_data.gas_limit),
            gas_price: Some(tx_data.gas_price.try_into().unwrap_or(u128::MAX)),
            max_fee_per_gas: tx_data.max_fee_per_gas.and_then(|v| v.try_into().ok()),
            max_priority_fee_per_gas: tx_data
                .max_priority_fee_per_gas
                .and_then(|v| v.try_into().ok()),
            nonce: None,
            access_list: tx_data.access_list.clone(),
            blob_versioned_hashes: tx_data.blob_versioned_hashes.clone(),
            max_fee_per_blob_gas: tx_data.max_fee_per_blob_gas.and_then(|v| v.try_into().ok()),
            signed_authorizations: tx_data.signed_authorizations.clone(),
        })
    }

    /// Get transaction with trace data (if available)
    /// For transactions that need internal transaction extraction
    pub async fn get_transaction_with_trace(&self, tx_hash: B256) -> Result<TransactionWithTrace> {
        // Get basic transaction data
        let transaction = self.get_transaction_by_hash(tx_hash).await?;
        let receipt = self.get_transaction_receipt(tx_hash).await?;

        // Get trace if transaction has input data (contract interaction)
        let trace = if !transaction.input.is_empty() && transaction.to.is_some() {
            // Try to get trace via RPC if available
            if let Some(_rpc_provider) = &self.rpc_provider {
                match self.get_trace_from_rpc(tx_hash).await {
                    Ok(trace) => Some(trace),
                    Err(e) => {
                        eprintln!("Failed to get trace from RPC: {}", e);
                        None
                    }
                }
            } else {
                // Fallback to simulation
                let call_request = self.build_unsigned_tx_from_tx_hash(tx_hash).await?;
                match self
                    .simulate_for_trace(call_request, transaction.block_number)
                    .await
                {
                    Ok(trace) => Some(trace),
                    Err(e) => {
                        eprintln!("Failed to simulate for trace: {}", e);
                        None
                    }
                }
            }
        } else {
            None
        };

        Ok(TransactionWithTrace {
            transaction,
            receipt,
            trace,
        })
    }

    /// Get trace from RPC using debug_traceTransaction
    async fn get_trace_from_rpc(&self, _tx_hash: B256) -> Result<TransactionTrace> {
        let _rpc_provider = self
            .rpc_provider
            .as_ref()
            .ok_or_else(|| eyre::eyre!("RPC provider not configured"))?;

        // Use debug_traceTransaction
        // Note: This is a placeholder - actual implementation depends on alloy version
        // and available RPC methods

        Err(eyre::eyre!("RPC trace not yet implemented"))
    }

    /// Simulate transaction to get trace
    async fn simulate_for_trace(
        &self,
        call_request: UnsignedTransaction,
        block_number: u64,
    ) -> Result<TransactionTrace> {
        // Simulate at block_number - 1 (state before transaction)
        let simulation_block = block_number.saturating_sub(1);

        // Use full trace simulation to get CallFrame
        let result = self
            .tx_simulator
            .simulate_unsigned_transaction_with_full_trace_at_block(
                call_request.clone(),
                simulation_block,
            )
            .await?;

        // Convert CallFrame from tx_simulator to our CallFrame type
        let mut call_frame = self.convert_call_frame(&result.call_trace);
        if let Some(gas) = call_request.gas {
            call_frame.gas_limit = gas;
        }

        Ok(TransactionTrace {
            call_frame,
            gas_used: result.gas_used,
            output: result.call_trace.output.clone().unwrap_or_default(),
            error: if !result.success {
                result.revert_reason
            } else {
                None
            },
        })
    }

    /// Convert tx_simulator CallFrame to our CallFrame type
    pub(super) fn convert_call_frame(&self, frame: &tx_simulator::types::CallFrame) -> CallFrame {
        self.convert_call_frame_at_depth(frame, 0)
    }

    fn convert_call_frame_at_depth(
        &self,
        frame: &tx_simulator::types::CallFrame,
        depth: u32,
    ) -> CallFrame {
        // tx_simulator uses alloy_rpc_types_trace::geth::CallFrame
        // which has different field names than our internal CallFrame
        CallFrame {
            from: frame.from,
            to: frame.to,
            value: frame.value.unwrap_or(U256::ZERO),
            input: frame.input.clone(),
            output: frame.output.clone().unwrap_or_default(),
            gas_used: frame.gas_used.try_into().unwrap_or(u64::MAX),
            gas_limit: frame.gas.try_into().unwrap_or(u64::MAX),
            depth,
            call_type: match frame.typ.to_ascii_uppercase().as_str() {
                "DELEGATECALL" => CallType::DelegateCall,
                "STATICCALL" => CallType::StaticCall,
                "CREATE" => CallType::Create,
                "CREATE2" => CallType::Create2,
                _ => CallType::Call,
            },
            subcalls: frame
                .calls
                .iter()
                .map(|c| self.convert_call_frame_at_depth(c, depth + 1))
                .collect(),
        }
    }

    /// Check if a transaction exists
    pub async fn transaction_exists(&self, tx_hash: B256) -> Result<bool> {
        self.tx_exists(tx_hash)
    }

    /// Check if transaction exists by hash
    pub fn tx_exists(&self, tx_hash: B256) -> Result<bool> {
        let provider = self.provider_factory.provider()?;
        Ok(provider.transaction_by_hash(tx_hash)?.is_some())
    }

    /// Calculate Txumber from block and transaction index
    pub fn calculate_tx_number(&self, block_number: u64, tx_index: u64) -> Result<u64> {
        let indices = self.get_block_tx_indices(block_number)?;
        Ok(indices.first_tx_num + tx_index)
    }

    /// Get transaction count for an address at a specific block
    pub async fn get_transaction_count(&self, address: Address, block: Option<u64>) -> Result<u64> {
        let block = block.unwrap_or(self.get_latest_block()?);
        let account = self.get_account(address, Some(block)).await?;
        Ok(account.nonce)
    }
}
