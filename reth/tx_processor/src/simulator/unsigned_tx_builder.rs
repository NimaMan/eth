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
use tx_simulator::UnsignedTransaction;
use eyre::Result;
use alloy_primitives::{Address, U256, B256, Bytes};
use crate::tx_processor::data_models::ProcessedTransaction;

/// Builds UnsignedTransaction objects from transaction data
pub struct UnsignedTxBuilder {
    transaction_loader: TransactionLoader,
}

impl UnsignedTxBuilder {
    /// Create new UnsignedTxBuilder with transaction loader
    pub fn new(transaction_loader: TransactionLoader) -> Self {
        Self {
            transaction_loader,
        }
    }

    /// Build an UnsignedTransaction from an existing ProcessedTransaction
    ///
    /// Useful when you want to re-simulate a transaction (or create a synthetic prior tx)
    /// using the fields already present in a ProcessedTransaction.
    ///
    /// Notes:
    /// - Gas limit is not stored in ProcessedTransaction, so we derive a conservative
    ///   limit from gas_used (x2, min 21000). You may override as needed.
    /// - EIP-1559 fees are respected if present (max_fee_per_gas / max_priority_fee).
    pub fn build_unsigned_from_processed_tx(ptx: &ProcessedTransaction) -> UnsignedTransaction {
        // Derive gas limit heuristically from gas_used
        let used = ptx.fees.gas_used;
        let gas_limit = used.saturating_mul(2).max(21_000);

        // Prefer EIP-1559 fields when available
        let (gas_price, max_fee, max_priority) = if let Some(mf) = ptx.fees.max_fee_per_gas {
            let mp = ptx.fees.max_priority_fee;
            (None, Some(mf.try_into().unwrap_or(0u128)), mp.map(|v| v.try_into().unwrap_or(0u128)))
        } else {
            // Legacy gas price from effective gas_price
            let gp_u128 = ptx.fees.gas_price.as_limbs()[0] as u128;
            (Some(gp_u128), None, None)
        };

        UnsignedTransaction {
            from: Some(ptx.from_address),
            to: ptx.to_address,
            value: Some(ptx.value),
            data: if ptx.input.is_empty() { None } else { Some(Bytes::from(ptx.input.clone())) },
            gas: Some(gas_limit),
            gas_price,
            max_fee_per_gas: max_fee,
            max_priority_fee_per_gas: max_priority,
            nonce: Some(ptx.nonce),
        }
    }
    
    /// Build UnsignedTransaction from transaction hash
    /// 
    /// This loads the transaction from the Reth database and converts it
    /// into an UnsignedTransaction that can be used for simulation
    pub async fn build_unsigned_transaction_from_tx_hash(&self, tx_hash: B256) -> Result<UnsignedTransaction> {
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
        ) = self.transaction_loader.load_transaction_data(tx_hash).await?;
        
        // Convert to UnsignedTransaction
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
            gas_price: Some(gas_price.as_limbs()[0] as u128), // Convert U256 to u128
            max_fee_per_gas: None, // EIP-1559 fields not available from old transactions
            max_priority_fee_per_gas: None,
            nonce: Some(nonce), // Include nonce for proper simulation
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
        }
    }
}
