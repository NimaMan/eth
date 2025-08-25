/// Direct contract storage access queries
/// 
/// This module handles direct reading from contract storage slots,
/// enabling low-level contract state inspection and analysis.

use pyo3::prelude::*;
use alloy_primitives::{Address, B256, U256};
use std::str::FromStr;

use super::main::PyChainQuery;

/// Read storage at a specific slot
/// 
/// Args:
///     address (str): Contract address
///     slot (str): Storage slot as hex string or number
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     str: Storage value as string
/// 
/// Examples:
///     # Read slot 2 (often total supply for ERC20)
///     value = query.get_storage_at(token_address, "2")
///     
///     # Read slot by hex
///     value = query.get_storage_at(contract, "0x0000000000000000000000000000000000000000000000000000000000000002")
pub fn get_storage_at(chain_query: &PyChainQuery, address: &str, slot: &str, block_number: Option<u64>) -> PyResult<String> {
    let addr = Address::from_str(address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid address: {}", e)
        ))?;
    
    let slot_key = if slot.starts_with("0x") {
        B256::from_str(slot.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid slot: {}", e)
            ))?
    } else {
        // Try to parse as number
        let slot_num = slot.parse::<u64>()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid slot number: {}", e)
            ))?;
        B256::from(U256::from(slot_num))
    };
    
    let query = chain_query.chain_query().clone();
    let value = chain_query.runtime().block_on(async move {
        query.get_storage_at(addr, slot_key, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to read storage: {}", e)
    ))?;
    
    Ok(value.to_string())
}