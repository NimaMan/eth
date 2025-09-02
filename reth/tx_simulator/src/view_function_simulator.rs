/// View Function Simulator - for calling view/pure functions on smart contracts
/// 
/// This module provides functionality to call view functions (read-only) on smart contracts
/// without needing to create a transaction. These are commonly used for querying token
/// balances, total supply, and other contract state.

use crate::{
    simulator::TxSimulator,
    types::ViewFunctionResult,
    call_simulator::UnsignedTransaction,
};
use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;

impl TxSimulator {
    /// Simulate a view function call (read-only contract call)
    /// 
    /// This simulates calling a view/pure function on a smart contract.
    /// No state changes are made, just returns the output data.
    /// 
    /// # Arguments
    /// * `contract` - The contract address to call
    /// * `data` - The encoded function call data (selector + args)
    /// * `block_number` - Optional block number to query at (defaults to latest)
    pub async fn simulate_view_function(
        &self,
        contract: Address,
        data: Bytes,
        block_number: Option<u64>,
    ) -> Result<ViewFunctionResult> {
        // Build a call request for the view function
        let unsigned_tx = UnsignedTransaction {
            from: Some(Address::ZERO), // View functions can be called from any address
            to: Some(contract),
            value: Some(U256::ZERO), // View functions shouldn't accept value
            data: Some(data),
            gas: Some(3_000_000), // Reasonable gas limit for view functions
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };
        
        // Get block number
        let block = if let Some(bn) = block_number {
            bn
        } else {
            self.get_latest_block()?
        };
        
        // We need to use the trace version to get the actual output data
        let result = self.simulate_unsigned_transaction_with_trace(unsigned_tx, Some(block)).await?;
        
        // Extract the output from the call trace
        let output = if result.success {
            // Get the output from the top-level call frame
            result.call_trace.output.clone().unwrap_or_default()
        } else {
            Bytes::new()
        };
        
        Ok(ViewFunctionResult {
            success: result.success,
            output,
            gas_used: result.gas_used,
        })
    }
    
}

// Public utility functions for view function encoding/decoding

/// Helper to encode a simple view function call (just selector, no args)
pub fn encode_view_function_call(selector: [u8; 4]) -> Bytes {
    Bytes::from(selector.to_vec())
}

/// Helper to encode a view function call with a single address argument
pub fn encode_view_function_with_address(selector: [u8; 4], address: Address) -> Bytes {
    let mut data = selector.to_vec();
    // Pad address to 32 bytes (addresses are left-padded with zeros)
    let mut padded = [0u8; 32];
    padded[12..].copy_from_slice(address.as_ref());
    data.extend_from_slice(&padded);
    Bytes::from(data)
}

/// Helper to decode a uint256 result from view function output
pub fn decode_uint256_result(output: &Bytes) -> U256 {
    if output.len() >= 32 {
        U256::from_be_slice(&output[..32])
    } else {
        U256::ZERO
    }
}

/// Helper to decode a uint8 result from view function output
pub fn decode_uint8_result(output: &Bytes) -> u8 {
    if output.len() >= 32 {
        output[31]
    } else {
        0
    }
}

/// Helper to decode a string result from view function output
pub fn decode_string_result(output: &Bytes) -> String {
    if output.len() < 64 {
        return String::new();
    }
    
    // Skip offset (32 bytes) and length (32 bytes)
    let len_bytes = &output[32..64];
    let len = U256::from_be_slice(len_bytes).to::<usize>();
    
    if output.len() < 64 + len {
        return String::new();
    }
    
    // Get the actual string bytes
    let string_bytes = &output[64..64 + len];
    String::from_utf8_lossy(string_bytes).to_string()
}

// ViewFunctionResult is already exported from lib.rs