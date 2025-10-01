use crate::tx_processor::data_models::ProcessedTransaction;
use crate::tx_processor::tx_loader::TransactionLoader;
use alloy_consensus::{EthereumTxEnvelope, TxEip4844};
use alloy_primitives::B256;
/// Signed Transaction Builder - Load signed transactions from DB or from a processed tx
///
/// Provides helpers to retrieve the original signed transaction for a given hash
/// and to convert a ProcessedTransaction back into the signed form by looking up
/// the transaction in the database.
use eyre::Result;

/// Builder for retrieving signed transactions
pub struct SignedTxBuilder {
    transaction_loader: TransactionLoader,
}

impl SignedTxBuilder {
    /// Create new SignedTxBuilder with a TransactionLoader
    pub fn new(transaction_loader: TransactionLoader) -> Self {
        Self { transaction_loader }
    }

    /// Load the original signed transaction by its hash
    pub async fn load_signed_transaction_by_hash(
        &self,
        tx_hash: B256,
    ) -> Result<EthereumTxEnvelope<TxEip4844>> {
        self.transaction_loader
            .load_signed_transaction_envelope_by_hash(tx_hash)
    }

    /// Load the original signed transaction for a given ProcessedTransaction
    ///
    /// Note: ProcessedTransaction stores the hash, but not the signature (v, r, s).
    /// To recover the signed transaction, we look it up by hash.
    pub async fn build_signed_from_processed(
        &self,
        ptx: &ProcessedTransaction,
    ) -> Result<EthereumTxEnvelope<TxEip4844>> {
        self.load_signed_transaction_by_hash(ptx.hash).await
    }
}
