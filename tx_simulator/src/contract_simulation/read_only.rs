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
