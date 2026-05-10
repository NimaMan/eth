/// Contract Method Simulator - for calling read-only contract methods
///
/// This module provides functionality to call read-only methods (view/pure functions) on smart contracts
/// without creating a transaction. These are commonly used for querying token balances,
/// total supply, decimals, and other contract state.
use crate::{
    simulator::TxSimulator,
    single_tx::unsigned::UnsignedTransaction,
    types::{ViewCallOverrides, ViewFunctionResult},
};
use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;

impl TxSimulator {
    /// Simulate a read-only contract method call (view/pure function)
    ///
    /// This simulates calling a view/pure function on a smart contract.
    /// No state changes are made, just returns the output data.
    ///
    /// # Arguments
    /// * `contract` - The contract address to call
    /// * `data` - The encoded function call data (selector + args)
    /// * `block_number` - Optional block number to query at (defaults to latest)
    /// * `overrides` - Optional overrides for from address / gas limit (defaults applied when None)
    pub async fn simulate_contract_read_only_call_with_options(
        &self,
        contract: Address,
        data: Bytes,
        block_number: Option<u64>,
        overrides: Option<ViewCallOverrides>,
    ) -> Result<ViewFunctionResult> {
        let resolved = overrides
            .unwrap_or_default()
            .resolve(&self.defaults.view_call);

        // Determine block we will query and derive the base fee for fee fields
        let block = block_number.unwrap_or(self.latest_historical_context_block_number()?);
        let base_fee = self.get_base_fee_at_block(block).unwrap_or(1);

        // Build a call request for the view function
        let mut unsigned_tx = UnsignedTransaction {
            from: Some(resolved.from),
            to: Some(contract),
            value: Some(U256::ZERO), // View functions shouldn't accept value
            data: Some(data),
            gas: Some(resolved.gas_limit),
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
            access_list: Vec::new(),
            blob_versioned_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            signed_authorizations: Vec::new(),
        };
        unsigned_tx.max_fee_per_gas = Some(base_fee);
        unsigned_tx.max_priority_fee_per_gas = Some(0);

        self.simulate_unsigned_transaction_for_output_at_block(unsigned_tx, block)
            .await
    }

    /// Convenience wrapper for simulate_contract_read_only_call_with_options
    pub async fn simulate_view_function(
        &self,
        contract: Address,
        data: Bytes,
        block_number: Option<u64>,
    ) -> Result<ViewFunctionResult> {
        self.simulate_contract_read_only_call_with_options(contract, data, block_number, None)
            .await
    }
}

// Public utility functions for contract method encoding/decoding

/// Encode a contract read-only call with no arguments (just 4-byte selector)
pub fn encode_contract_read_call_no_args(selector: [u8; 4]) -> Bytes {
    Bytes::from(selector.to_vec())
}

/// Encode a contract read-only call with a single address argument (e.g., balanceOf)
pub fn encode_contract_read_call_with_address_arg(selector: [u8; 4], address: Address) -> Bytes {
    let mut data = selector.to_vec();
    // Pad address to 32 bytes (addresses are left-padded with zeros)
    let mut padded = [0u8; 32];
    padded[12..].copy_from_slice(address.as_ref());
    data.extend_from_slice(&padded);
    Bytes::from(data)
}

/// Decode a uint256 value from contract method output bytes
pub fn decode_uint256_from_contract_output(output: &Bytes) -> U256 {
    if output.len() >= 32 {
        U256::from_be_slice(&output[..32])
    } else {
        U256::ZERO
    }
}

/// Decode a uint8 value from contract method output bytes (e.g., decimals)
pub fn decode_uint8_from_contract_output(output: &Bytes) -> u8 {
    if output.len() >= 32 {
        output[31]
    } else {
        0
    }
}

/// Decode a string value from contract method output bytes (e.g., name, symbol)
pub fn decode_string_from_contract_output(output: &Bytes) -> String {
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
