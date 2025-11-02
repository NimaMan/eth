use crate::tx_processor::data_models::{ProcessedAccessListItem, ProcessedTransaction};
/// Unsigned Transaction Builder - Converts transaction data to UnsignedTransaction for simulation
///
/// OBJECTIVE: Build UnsignedTransaction objects from transaction data loaded from Reth database
///
/// This module handles:
/// 1. Loading transaction data from the database via TransactionLoader
/// 2. Converting raw transaction data into tx_simulator::UnsignedTransaction
/// 3. Setting appropriate gas parameters and nonces for simulation
///
/// Flow: TX Hash → Load from DB → Build UnsignedTransaction → Ready for simulation
use crate::tx_processor::tx_loader::TransactionLoader;
use alloy_eips::{eip2930::AccessListItem, eip7702::SignedAuthorization};
use alloy_primitives::{Address, Bytes, B256, U256};
use eyre::Result;
use std::convert::TryInto;
use tx_simulator::UnsignedTransaction;

/// Builds UnsignedTransaction objects from transaction data
pub struct UnsignedTxBuilder {
    transaction_loader: TransactionLoader,
}

impl UnsignedTxBuilder {
    /// Create new UnsignedTxBuilder with transaction loader
    pub fn new(transaction_loader: TransactionLoader) -> Self {
        Self { transaction_loader }
    }

    /// Build an UnsignedTransaction from an existing ProcessedTransaction
    ///
    /// Useful when you want to re-simulate a transaction (or create a synthetic prior tx)
    /// using the fields already present in a ProcessedTransaction.
    ///
    /// Notes:
    /// - Gas limit is read from ProcessedTransaction.fees; callers may override as needed.
    /// - EIP-1559 fees are respected if present (max_fee_per_gas / max_priority_fee).
    pub fn build_unsigned_from_processed_tx(ptx: &ProcessedTransaction) -> UnsignedTransaction {
        // Expect processed payloads to carry the original gas limit
        let gas_limit = ptx.fees.gas_limit;
        assert!(gas_limit > 0, "ProcessedTransaction missing gas_limit");

        // Prefer EIP-1559 fields when available
        let gas_price_u128 = ptx.fees.gas_price.try_into().ok();
        let max_fee_u128 = ptx.fees.max_fee_per_gas.and_then(|v| v.try_into().ok());
        let max_priority_u128 = ptx.fees.max_priority_fee.and_then(|v| v.try_into().ok());
        let (gas_price, max_fee, max_priority) = match ptx.raw_tx_type {
            0 | 1 => (gas_price_u128, None, None),
            2 | 3 | 4 => (None, max_fee_u128, max_priority_u128),
            _ => (gas_price_u128, max_fee_u128, max_priority_u128),
        };
        let access_list: Vec<AccessListItem> = ptx
            .access_list
            .iter()
            .map(|item| AccessListItem {
                address: item.address,
                storage_keys: item.storage_keys.clone(),
            })
            .collect();
        let max_fee_per_blob_gas = ptx
            .max_fee_per_blob_gas
            .and_then(|v| u128::try_from(v).ok());

        UnsignedTransaction {
            from: Some(ptx.from_address),
            to: ptx.to_address,
            value: Some(ptx.value),
            data: if ptx.input.is_empty() {
                None
            } else {
                Some(Bytes::from(ptx.input.clone()))
            },
            gas: Some(gas_limit),
            gas_price,
            max_fee_per_gas: max_fee,
            max_priority_fee_per_gas: max_priority,
            nonce: None,
            access_list,
            blob_versioned_hashes: ptx.blob_versioned_hashes.clone(),
            max_fee_per_blob_gas,
            signed_authorizations: ptx.signed_authorizations.clone(),
        }
    }

    /// Build UnsignedTransaction from transaction hash
    ///
    /// This loads the transaction from the Reth database and converts it
    /// into an UnsignedTransaction that can be used for simulation
    pub async fn build_unsigned_transaction_from_tx_hash(
        &self,
        tx_hash: B256,
    ) -> Result<UnsignedTransaction> {
        // Load transaction data from database - returns a tuple
        let (
            _tx_hash,
            _block_number,
            _timestamp,
            _tx_index,
            from,
            to,
            value,
            input,
            gas_price,
            _gas_used,
            _status,
            nonce,
            _logs,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            access_list,
            blob_versioned_hashes,
            max_fee_per_blob_gas,
            signed_authorizations,
            raw_tx_type,
        ) = self
            .transaction_loader
            .load_transaction_data(tx_hash)
            .await?;

        // Convert to UnsignedTransaction
        let gas_price_u128 = gas_price.try_into().ok();
        let max_fee_u128 = max_fee_per_gas.and_then(|v| v.try_into().ok());
        let max_priority_u128 = max_priority_fee_per_gas.and_then(|v| v.try_into().ok());
        let (final_gas_price, final_max_fee, final_max_priority) = match raw_tx_type {
            0 | 1 => (gas_price_u128, None, None),
            2 | 3 | 4 => (None, max_fee_u128, max_priority_u128),
            _ => (gas_price_u128, max_fee_u128, max_priority_u128),
        };
        let access_list: Vec<AccessListItem> = access_list
            .into_iter()
            .map(|item| AccessListItem {
                address: item.address,
                storage_keys: item.storage_keys,
            })
            .collect();
        let max_fee_per_blob_gas = max_fee_per_blob_gas.and_then(|v| u128::try_from(v).ok());

        let unsigned_tx = UnsignedTransaction {
            from: Some(from),
            to,
            value: Some(value),
            data: if input.is_empty() {
                None
            } else {
                Some(Bytes::from(input))
            },
            gas: Some(gas_limit),
            gas_price: final_gas_price,
            max_fee_per_gas: final_max_fee,
            max_priority_fee_per_gas: final_max_priority,
            nonce: Some(nonce), // Include nonce for proper simulation
            access_list,
            blob_versioned_hashes,
            max_fee_per_blob_gas,
            signed_authorizations,
        };

        Ok(unsigned_tx)
    }

    /// Build UnsignedTransaction from raw transaction parameters
    ///
    /// This is useful when you already have the transaction parameters
    /// and don't need to load from the database
    pub fn build_unsigned_transaction_from_params(
        from: Address,
        to: Option<Address>,
        value: U256,
        data: Vec<u8>,
        gas_limit: u64,
        gas_price: u128,
        nonce: u64,
    ) -> UnsignedTransaction {
        UnsignedTransaction {
            from: Some(from),
            to,
            value: Some(value),
            data: if data.is_empty() {
                None
            } else {
                Some(Bytes::from(data))
            },
            gas: Some(gas_limit),
            gas_price: Some(gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: Some(nonce),
            access_list: Vec::new(),
            blob_versioned_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            signed_authorizations: Vec::new(),
        }
    }
}
