// Conversion utilities for transforming between different transaction representations

use eyre::{Result, eyre};
use ethers::types::{Transaction as EthersTransaction, U256, Bytes};
use crate::mempool_fetcher::MempoolTransaction;
use tx_simulator::UnsignedTransaction;
use alloy_primitives::{Address, U256 as AlloyU256, Bytes as AlloyBytes};
use serde_json::Value;

/// Convert a MempoolTransaction (from IPC) to an ethers Transaction view
/// This is needed for compatibility with existing analysis code that expects ethers types
pub fn convert_nonblocking_to_transaction_view(tx: &MempoolTransaction) -> Result<EthersTransaction> {
    // The MempoolTransaction contains transaction data as a JSON Value
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

/// Convert IPC transaction data to UnsignedTransaction with proper EIP-1559 gas parameter handling
/// This function ensures that priority fees never exceed max fees, preventing simulation errors
pub fn ipc_to_call_request(ipc_tx: &Value) -> Result<UnsignedTransaction> {
    // Parse gas price fields - handle both legacy and EIP-1559 transactions
    let gas_price = ipc_tx["gasPrice"].as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|s| u128::from_str_radix(s, 16).ok());
    
    let max_fee_per_gas = ipc_tx["maxFeePerGas"].as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|s| u128::from_str_radix(s, 16).ok());
    
    let max_priority_fee_per_gas = ipc_tx["maxPriorityFeePerGas"].as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|s| u128::from_str_radix(s, 16).ok());
    
    // Determine transaction type and set appropriate gas parameters
    let (final_gas_price, final_max_fee, final_max_priority) = 
        match (gas_price, max_fee_per_gas, max_priority_fee_per_gas) {
            // EIP-1559 transaction with both fields
            (_, Some(max_fee), Some(max_priority)) => {
                // Ensure priority fee doesn't exceed max fee (safety check)
                let safe_priority = max_priority.min(max_fee);
                (None, Some(max_fee), Some(safe_priority))
            }
            // EIP-1559 with only max_fee (missing priority fee)
            (_, Some(max_fee), None) => {
                // Use a conservative default priority fee (0.1 Gwei)
                let default_priority = 100_000_000u128; // 0.1 Gwei
                let safe_priority = default_priority.min(max_fee);
                (None, Some(max_fee), Some(safe_priority))
            }
            // Legacy transaction
            (Some(price), None, None) => {
                (Some(price), None, None)
            }
            // Invalid combinations - default to legacy with standard gas price
            _ => {
                // This handles edge cases like priority without max_fee
                let default_gas_price = gas_price.unwrap_or(20_000_000_000); // 20 Gwei default
                (Some(default_gas_price), None, None)
            }
        };
    
    Ok(UnsignedTransaction {
        from: ipc_tx["from"].as_str()
            .and_then(|s| s.parse::<Address>().ok()),
        to: ipc_tx["to"].as_str()
            .filter(|s| !s.is_empty() && *s != "null")
            .and_then(|s| s.parse::<Address>().ok()),
        gas: ipc_tx["gas"].as_str()
            .and_then(|s| s.strip_prefix("0x"))
            .and_then(|s| u64::from_str_radix(s, 16).ok()),
        gas_price: final_gas_price,
        max_fee_per_gas: final_max_fee,
        max_priority_fee_per_gas: final_max_priority,
        value: ipc_tx["value"].as_str()
            .and_then(|s| AlloyU256::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16).ok()),
        data: ipc_tx["input"].as_str()
            .and_then(|s| hex::decode(s.strip_prefix("0x").unwrap_or(s)).ok())
            .map(AlloyBytes::from),
        nonce: ipc_tx["nonce"].as_str()
            .and_then(|s| s.strip_prefix("0x"))
            .and_then(|s| u64::from_str_radix(s, 16).ok()),
    })
}

/// Alias for ipc_to_call_request - kept for backwards compatibility
pub fn ipc_to_unsigned_tx(ipc_tx: &Value) -> Result<UnsignedTransaction> {
    ipc_to_call_request(ipc_tx)
}