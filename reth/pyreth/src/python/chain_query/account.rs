/// Account-related chain queries
/// 
/// This module handles all account-related blockchain queries including:
/// - ETH balance queries
/// - Account nonce (transaction count)
/// - Contract code detection
/// - Comprehensive account information

use pyo3::prelude::*;
use pyo3::types::PyDict;
use alloy_primitives::Address;
use std::str::FromStr;

use super::main::PyChainQuery;

/// Get ETH balance for an address
/// 
/// Args:
///     address (str): Ethereum address
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     str: Balance in wei as string
pub fn get_balance(chain_query: &PyChainQuery, address: &str, block_number: Option<u64>) -> PyResult<String> {
    let addr = Address::from_str(address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid address: {}", e)
        ))?;
    
    let query = chain_query.chain_query().clone();
    let balance = chain_query.runtime().block_on(async move {
        query.get_balance(addr, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get balance: {}", e)
    ))?;
    
    Ok(balance.to_string())
}

/// Get nonce for an address
/// 
/// Args:
///     address (str): Ethereum address
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     int: Account nonce
pub fn get_nonce(chain_query: &PyChainQuery, address: &str, block_number: Option<u64>) -> PyResult<u64> {
    let addr = Address::from_str(address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid address: {}", e)
        ))?;
    
    let query = chain_query.chain_query().clone();
    chain_query.runtime().block_on(async move {
        query.get_nonce(addr, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get nonce: {}", e)
    ))
}

/// Check if an address has code (is a contract)
/// 
/// Args:
///     address (str): Ethereum address
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     bool: True if address is a contract, False if EOA
pub fn has_code(chain_query: &PyChainQuery, address: &str, block_number: Option<u64>) -> PyResult<bool> {
    let addr = Address::from_str(address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid address: {}", e)
        ))?;
    
    let query = chain_query.chain_query().clone();
    chain_query.runtime().block_on(async move {
        query.has_code(addr, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to check code: {}", e)
    ))
}

/// Get complete account information
/// 
/// Args:
///     address (str): Ethereum address
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     dict: Account info with balance, nonce, and has_code
pub fn get_account_info(chain_query: &PyChainQuery, py: Python, address: &str, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
    let addr = Address::from_str(address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid address: {}", e)
        ))?;
    
    let query = chain_query.chain_query().clone();
    let info = chain_query.runtime().block_on(async move {
        query.account.get_account_info(addr, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get account info: {}", e)
    ))?;
    
    let dict = PyDict::new(py);
    dict.set_item("address", crate::utils::to_checksum_address(&info.address))?;
    dict.set_item("balance", info.balance.to_string())?;
    dict.set_item("nonce", info.nonce)?;
    dict.set_item("has_code", info.has_code)?;
    
    Ok(dict.into())
}