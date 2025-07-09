// Conversion utilities for transforming between different transaction representations

use eyre::{Result, eyre};
use ethers::types::{Transaction as EthersTransaction, U256, H160, Bytes};
use crate::mempool_fetcher::NonBlockingTransaction;

/// Convert a NonBlockingTransaction (from IPC) to an ethers Transaction view
/// This is needed for compatibility with existing analysis code that expects ethers types
pub fn convert_nonblocking_to_transaction_view(tx: &NonBlockingTransaction) -> Result<EthersTransaction> {
    // The NonBlockingTransaction contains transaction data as a JSON Value
    let data = &tx.data;
    
    // Create a minimal transaction view with the fields we need
    let mut ethers_tx = EthersTransaction::default();
    
    // Set hash
    ethers_tx.hash = tx.hash.parse()?;
    
    // Extract from address
    let from_str = data.get("from")
        .and_then(|v| v.as_str())
        .ok_or_else(|| eyre!("Missing 'from' field"))?;
    ethers_tx.from = from_str.parse()?;
    
    // Handle optional to address (None for contract creation)
    if let Some(to_val) = data.get("to") {
        if let Some(to_str) = to_val.as_str() {
            ethers_tx.to = Some(to_str.parse()?);
        }
    }
    
    // Set value
    let value_str = data.get("value")
        .and_then(|v| v.as_str())
        .unwrap_or("0x0");
    ethers_tx.value = U256::from_str_radix(value_str.trim_start_matches("0x"), 16)?;
    
    // Set gas price if available
    if let Some(gas_price_str) = data.get("gasPrice").and_then(|v| v.as_str()) {
        ethers_tx.gas_price = Some(U256::from_str_radix(gas_price_str.trim_start_matches("0x"), 16)?);
    }
    
    // Set input data
    let input_str = data.get("input")
        .and_then(|v| v.as_str())
        .unwrap_or("0x");
    ethers_tx.input = Bytes::from(hex::decode(input_str.trim_start_matches("0x"))?);
    
    // Set nonce if available
    if let Some(nonce_str) = data.get("nonce").and_then(|v| v.as_str()) {
        ethers_tx.nonce = U256::from_str_radix(nonce_str.trim_start_matches("0x"), 16)?;
    }
    
    // Set gas if available
    if let Some(gas_str) = data.get("gas").and_then(|v| v.as_str()) {
        ethers_tx.gas = U256::from_str_radix(gas_str.trim_start_matches("0x"), 16)?;
    }
    
    Ok(ethers_tx)
}