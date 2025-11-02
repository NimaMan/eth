use alloy_consensus::Transaction as _;
use alloy_consensus::{
    transaction::{SignerRecoverable, TransactionMeta, TxType},
    EthereumTxEnvelope, TxEip4844, Typed2718,
};
use alloy_eips::{eip2930::AccessListItem, eip7702::SignedAuthorization};
use alloy_primitives::{Log as AlloyLog, TxKind, B256, U256};
use eyre::Result;
use reth_chainspec::ChainSpecBuilder;
use reth_db::{mdbx::DatabaseArguments, open_db_read_only, ClientVersion, DatabaseEnv};
use reth_node_ethereum::EthereumNode;
use reth_node_types::NodeTypesWithDBAdapter;
/// Transaction Loader - Fetches transaction data from Reth DB
///
/// This module provides transaction loading that fetches data from Reth's database
use reth_provider::{
    providers::StaticFileProvider, BlockReader, ProviderFactory, ReceiptProvider,
    TransactionsProvider,
};
use std::{cmp, path::Path, sync::Arc};
use tracing::info;

/// Transaction Loader that fetches from Reth database
#[derive(Clone)]
pub struct TransactionLoader {
    provider_factory: ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>,
}

impl TransactionLoader {
    /// Create a new transaction loader
    pub fn new(reth_datadir: &str) -> Result<Self> {
        // Initialize database like reth_tx_simulator does
        let db_path = Path::new(reth_datadir).join("db");
        let static_files_path = Path::new(reth_datadir).join("static_files");

        let db = Arc::new(open_db_read_only(
            &db_path,
            DatabaseArguments::new(ClientVersion::default()),
        )?);

        let chain_spec = Arc::new(ChainSpecBuilder::mainnet().build());

        let provider_factory =
            ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>::new(
                db.clone(),
                chain_spec.clone(),
                StaticFileProvider::read_only(static_files_path, false)?, // Don't watch files - Reth is already watching
            );

        Ok(Self { provider_factory })
    }

    /// Load the original signed transaction by hash
    pub fn load_signed_transaction_envelope_by_hash(
        &self,
        tx_hash: B256,
    ) -> Result<EthereumTxEnvelope<TxEip4844>> {
        let provider = self.provider_factory.provider()?;
        let (tx, _meta) = provider
            .transaction_by_hash_with_meta(tx_hash)?
            .ok_or_else(|| eyre::eyre!("Transaction not found: {}", tx_hash))?;
        Ok(tx)
    }

    /// Create a new transaction loader with an existing provider factory
    /// This is useful when sharing a database connection across multiple components
    pub fn with_provider_factory(
        provider_factory: ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>,
    ) -> Result<Self> {
        Ok(Self { provider_factory })
    }

    /// Load transaction data by hash from the database
    /// Returns data needed for process_transaction
    pub async fn load_transaction_data(
        &self,
        tx_hash: B256,
    ) -> Result<(
        B256,                              // tx_hash
        u64,                               // block_number
        u64,                               // timestamp
        u64,                               // tx_index
        alloy_primitives::Address,         // from
        Option<alloy_primitives::Address>, // to
        U256,                              // value
        Vec<u8>,                           // input
        U256,                              // gas_price
        u64,                               // gas_used
        bool,                              // status
        u64,                               // nonce
        Vec<AlloyLog>,                     // logs
        u64,                               // gas_limit
        Option<U256>,                      // max_fee_per_gas
        Option<U256>,                      // max_priority_fee_per_gas
        Vec<AccessListItem>,               // access list
        Vec<B256>,                         // blob versioned hashes
        Option<U256>,                      // max_fee_per_blob_gas
        Vec<SignedAuthorization>,          // signed authorizations
        u8,                                // raw transaction type
    )> {
        // Get provider
        let provider = self.provider_factory.provider()?;

        // Get transaction with metadata
        let (tx, meta) = provider
            .transaction_by_hash_with_meta(tx_hash)?
            .ok_or_else(|| eyre::eyre!("Transaction not found: {}", tx_hash))?;

        // Get receipt
        let receipt = provider
            .receipt_by_hash(tx_hash)?
            .ok_or_else(|| eyre::eyre!("Receipt not found: {}", tx_hash))?;

        // Get all receipts for the block to calculate actual gas used
        let receipts = provider
            .receipts_by_block(meta.block_hash.into())?
            .ok_or_else(|| eyre::eyre!("Receipts not found for block"))?;

        // Get block header by number
        let block = provider
            .block_by_number(meta.block_number)?
            .ok_or_else(|| eyre::eyre!("Block not found: {}", meta.block_number))?
            .header;

        // Extract transaction details
        let from = tx
            .recover_signer()
            .map_err(|_| eyre::eyre!("Failed to recover signer for {:?}", tx_hash))?;

        // Get transaction data directly from transaction methods (new API)
        let to = tx.to();
        let value = tx.value();
        let input = tx.input().to_vec();
        let nonce = tx.nonce();
        let base_fee_per_gas = block.base_fee_per_gas.unwrap_or_default() as u128;
        let gas_limit = tx.gas_limit();
        let tx_type = tx.tx_type();
        let max_fee_value = tx.max_fee_per_gas();
        let max_priority_value = tx.max_priority_fee_per_gas();
        let gas_price = match tx_type {
            TxType::Legacy | TxType::Eip2930 => U256::from(max_fee_value),
            _ => {
                let max_priority = max_priority_value.unwrap_or(0);
                let effective_priority =
                    cmp::min(max_priority, max_fee_value.saturating_sub(base_fee_per_gas));
                U256::from(base_fee_per_gas + effective_priority)
            }
        };
        // Calculate actual gas used for this transaction
        let actual_gas_used = if meta.index == 0 {
            receipt.cumulative_gas_used
        } else {
            let prev_receipt = receipts
                .get(meta.index as usize - 1)
                .ok_or_else(|| eyre::eyre!("Previous receipt not found"))?;
            receipt.cumulative_gas_used - prev_receipt.cumulative_gas_used
        };
        let gas_used = actual_gas_used;

        let raw_tx_type = tx.ty();
        let (max_fee_per_gas_opt, max_priority_fee_per_gas_opt) = match tx_type {
            TxType::Eip1559 | TxType::Eip4844 | TxType::Eip7702 => (
                Some(U256::from(max_fee_value)),
                max_priority_value.map(U256::from),
            ),
            _ => (None, None),
        };

        // Convert status
        let status = receipt.success;

        // Convert logs to AlloyLog format
        let logs: Vec<AlloyLog> = receipt
            .logs
            .into_iter()
            .map(|log| {
                AlloyLog::new_unchecked(log.address, log.topics().to_vec(), log.data.data.clone())
            })
            .collect();

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

        Ok((
            tx_hash,
            meta.block_number,
            block.timestamp,
            meta.index as u64,
            from,
            to,
            value,
            input,
            gas_price,
            gas_used,
            status,
            nonce,
            logs,
            gas_limit,
            max_fee_per_gas_opt,
            max_priority_fee_per_gas_opt,
            access_list,
            blob_versioned_hashes,
            max_fee_per_blob_gas,
            signed_authorizations,
            raw_tx_type,
        ))
    }
}
