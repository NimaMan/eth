/// Call Data Builder - Converts transaction data to CallRequest for simulation
/// 
/// OBJECTIVE: Build CallRequest objects from transaction data loaded from Reth database
/// 
/// This module handles:
/// 1. Loading transaction data from the database via TransactionLoader
/// 2. Converting raw transaction data into tx_simulator::CallRequest
/// 3. Setting appropriate gas parameters and nonces for simulation
/// 
/// Flow: TX Hash → Load from DB → Build CallRequest → Ready for simulation

use crate::tx_processor::tx_loader::TransactionLoader;
use tx_simulator::CallRequest;
use eyre::Result;
use alloy_primitives::{Address, U256, B256, Bytes};

/// Builds CallRequest objects from transaction data
pub struct CallDataBuilder {
    transaction_loader: TransactionLoader,
}

impl CallDataBuilder {
    /// Create new CallDataBuilder with transaction loader
    pub fn new(transaction_loader: TransactionLoader) -> Self {
        Self {
            transaction_loader,
        }
    }
    
    /// Build CallRequest from transaction hash
    /// 
    /// This loads the transaction from the Reth database and converts it
    /// into a CallRequest that can be used for simulation
    pub async fn build_call_request_from_tx_hash(&self, tx_hash: B256) -> Result<CallRequest> {
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
        
        // Convert to CallRequest
        let call_request = CallRequest {
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
        
        Ok(call_request)
    }
    
    /// Build CallRequest from raw transaction parameters
    /// 
    /// This is useful when you already have the transaction parameters
    /// and don't need to load from the database
    pub fn build_call_request_from_params(
        from: Address,
        to: Option<Address>,
        value: U256,
        data: Vec<u8>,
        gas_limit: u64,
        gas_price: u128,
        nonce: u64,
    ) -> CallRequest {
        CallRequest {
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
    
    /// Build CallRequest with default gas parameters for testing
    pub fn build_call_request_simple(
        from: Address,
        to: Option<Address>,
        value: U256,
        data: Vec<u8>,
    ) -> CallRequest {
        CallRequest {
            from: Some(from),
            to,
            value: Some(value),
            data: if data.is_empty() { 
                None 
            } else { 
                Some(Bytes::from(data))
            },
            gas: Some(300_000), // Default gas limit
            gas_price: Some(20_000_000_000), // Default 20 gwei
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: Some(0), // Default nonce
        }
    }
}