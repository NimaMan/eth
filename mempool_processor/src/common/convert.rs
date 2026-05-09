// Conversion utilities for transforming between different transaction representations

use alloy_eips::{eip2930::AccessListItem, eip7702::SignedAuthorization};
use alloy_primitives::{Address, Bytes as AlloyBytes, B256, U256 as AlloyU256};
use eyre::Result;
use serde_json::Value;
use tx_simulator::UnsignedTransaction;

/// Convert IPC transaction data to UnsignedTransaction with proper EIP-1559 gas parameter handling
/// This function ensures that priority fees never exceed max fees, preventing simulation errors
pub fn ipc_to_call_request(ipc_tx: &Value) -> Result<UnsignedTransaction> {
    // Parse gas price fields - handle both legacy and EIP-1559 transactions
    let gas_price = ipc_tx["gasPrice"]
        .as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|s| u128::from_str_radix(s, 16).ok());

    let max_fee_per_gas = ipc_tx["maxFeePerGas"]
        .as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|s| u128::from_str_radix(s, 16).ok());

    let max_priority_fee_per_gas = ipc_tx["maxPriorityFeePerGas"]
        .as_str()
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
            (Some(price), None, None) => (Some(price), None, None),
            // Invalid combinations - default to legacy with standard gas price
            _ => {
                // This handles edge cases like priority without max_fee
                let default_gas_price = gas_price.unwrap_or(20_000_000_000); // 20 Gwei default
                (Some(default_gas_price), None, None)
            }
        };

    let access_list = ipc_tx
        .get("accessList")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|entry| {
                    let address = entry
                        .get("address")
                        .and_then(|v| v.as_str())
                        .and_then(|addr| addr.parse::<Address>().ok())?;
                    let storage_keys = entry
                        .get("storageKeys")
                        .and_then(|v| v.as_array())
                        .map(|keys| {
                            keys.iter()
                                .filter_map(|key| key.as_str())
                                .filter_map(|key| key.parse::<B256>().ok())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    Some(AccessListItem {
                        address,
                        storage_keys,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let blob_versioned_hashes = ipc_tx
        .get("blobVersionedHashes")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .filter_map(|hash| hash.parse::<B256>().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let max_fee_per_blob_gas = ipc_tx
        .get("maxFeePerBlobGas")
        .and_then(|v| v.as_str())
        .and_then(|s| {
            let trimmed = s.strip_prefix("0x").unwrap_or(s);
            u128::from_str_radix(trimmed, 16).ok()
        });

    Ok(UnsignedTransaction {
        from: ipc_tx["from"]
            .as_str()
            .and_then(|s| s.parse::<Address>().ok()),
        to: ipc_tx["to"]
            .as_str()
            .filter(|s| !s.is_empty() && *s != "null")
            .and_then(|s| s.parse::<Address>().ok()),
        gas: ipc_tx["gas"]
            .as_str()
            .and_then(|s| s.strip_prefix("0x"))
            .and_then(|s| u64::from_str_radix(s, 16).ok()),
        gas_price: final_gas_price,
        max_fee_per_gas: final_max_fee,
        max_priority_fee_per_gas: final_max_priority,
        value: ipc_tx["value"]
            .as_str()
            .and_then(|s| AlloyU256::from_str_radix(s.strip_prefix("0x").unwrap_or(s), 16).ok()),
        data: ipc_tx["input"]
            .as_str()
            .and_then(|s| hex::decode(s.strip_prefix("0x").unwrap_or(s)).ok())
            .map(AlloyBytes::from),
        nonce: ipc_tx["nonce"]
            .as_str()
            .and_then(|s| s.strip_prefix("0x"))
            .and_then(|s| u64::from_str_radix(s, 16).ok()),
        access_list,
        blob_versioned_hashes,
        max_fee_per_blob_gas,
        signed_authorizations: Vec::<SignedAuthorization>::new(),
    })
}

/// Alias for ipc_to_call_request - kept for backwards compatibility
pub fn ipc_to_unsigned_tx(ipc_tx: &Value) -> Result<UnsignedTransaction> {
    ipc_to_call_request(ipc_tx)
}
