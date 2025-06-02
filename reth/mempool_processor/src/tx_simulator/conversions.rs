// Conversions for mempool_processor specific types to REVM types

use crate::mempool_processor::types::TransactionView;
use revm_primitives::{
    Address as RevmAddress, Bytes as RevmBytes, 
    U256 as RevmU256
};
use revm_context::{TxEnv as RevmTxEnv_ctx, TransactTo as RevmTransactTo_ctx};
use revm_tx_simulator_lib::conversions::ethers_to_revm_u256 as _ethers_to_revm_u256; // aliased as unused for now
use eyre::Result;

const DEFAULT_GAS_LIMIT_FOR_MISSING: u64 = 30_000_000; // A high default
const DEFAULT_NONCE_FOR_MISSING: u64 = 0; // Default nonce if not provided

// Helper for U256 to u128 conversion, acknowledging potential truncation
fn revm_u256_to_u128_lossy(val: RevmU256) -> u128 {
    val.into_limbs()[0] as u128 // Takes the lowest limb, effectively a modulo operation if val > u128::MAX
}

/// Converts a TransactionView (our internal simplified format) to a REVM TxEnv.
pub fn transaction_view_to_revm_tx_env(
    tx_view: &TransactionView,
    cfg_chain_id: u64, // chain_id from CfgEnv
) -> Result<RevmTxEnv_ctx> {
    let caller = RevmAddress::from_slice(&tx_view.from);
    
    let transact_to_revm = match &tx_view.to {
        Some(to_addr) => RevmTransactTo_ctx::Call(RevmAddress::from_slice(to_addr)),
        None => RevmTransactTo_ctx::Create,
    };

    let value_revm = _ethers_to_revm_u256(tx_view.value);

    let gas_limit_u64 = tx_view.gas_limit.map_or(DEFAULT_GAS_LIMIT_FOR_MISSING, |gl| gl.as_u64());
    
    let gas_price_u128 = tx_view.gas_price.map_or(0_u128, |gp| revm_u256_to_u128_lossy(_ethers_to_revm_u256(gp)));

    let nonce_u64 = tx_view.nonce.map_or(DEFAULT_NONCE_FOR_MISSING, |n| _ethers_to_revm_u256(n).into_limbs()[0]);

    let input_data_bytes = tx_view.input_data.clone().unwrap_or_default();

    Ok(RevmTxEnv_ctx {
        caller,
        gas_limit: gas_limit_u64,
        gas_price: gas_price_u128, // Corrected to u128
        gas_priority_fee: None, // TxEnv field is Option<u128>, TransactionView doesn't have it
        kind: transact_to_revm,
        value: value_revm,
        data: RevmBytes::from(input_data_bytes),
        chain_id: Some(cfg_chain_id),
        nonce: nonce_u64, // Corrected to u64
        access_list: revm_context::transaction::AccessList(Vec::new()),
        max_fee_per_blob_gas: 0_u128, // TxEnv field is u128, TransactionView doesn't have it, default to 0
        blob_hashes: Vec::new(),
        tx_type: 0, // Default to legacy
        authorization_list: Vec::new(),
    })
} 