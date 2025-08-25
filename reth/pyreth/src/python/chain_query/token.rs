/// ERC20 token-related chain queries
/// 
/// This module handles all ERC20 token queries including:
/// - Token balance queries
/// - Token metadata (name, symbol, decimals)
/// - Total supply queries
/// - Allowance queries
/// - Comprehensive token information

use pyo3::prelude::*;
use pyo3::types::PyDict;
use alloy_primitives::Address;
use std::str::FromStr;

use super::main::PyChainQuery;

/// Get ERC20 token balance for an address
/// 
/// Args:
///     token_address (str): ERC20 token contract address
///     holder_address (str): Address to check balance for
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     str: Token balance as string (in token's smallest unit)
pub fn get_token_balance(chain_query: &PyChainQuery, token_address: &str, holder_address: &str, block_number: Option<u64>) -> PyResult<String> {
    let token = Address::from_str(token_address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid token address: {}", e)
        ))?;
    
    let holder = Address::from_str(holder_address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid holder address: {}", e)
        ))?;
    
    let query = chain_query.chain_query().clone();
    let balance = chain_query.runtime().block_on(async move {
        query.get_token_balance(token, holder, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get token balance: {}", e)
    ))?;
    
    Ok(balance.to_string())
}

/// Get ERC20 token total supply
/// 
/// Args:
///     token_address (str): ERC20 token contract address
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     str: Total supply as string
pub fn get_token_total_supply(chain_query: &PyChainQuery, token_address: &str, block_number: Option<u64>) -> PyResult<String> {
    let token = Address::from_str(token_address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid token address: {}", e)
        ))?;
    
    let query = chain_query.chain_query().clone();
    let supply = chain_query.runtime().block_on(async move {
        query.get_token_total_supply(token, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get total supply: {}", e)
    ))?;
    
    Ok(supply.to_string())
}

/// Get ERC20 token decimals
/// 
/// Args:
///     token_address (str): ERC20 token contract address
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     int: Token decimals (defaults to 18 if not found)
pub fn get_token_decimals(chain_query: &PyChainQuery, token_address: &str, block_number: Option<u64>) -> PyResult<u8> {
    let token = Address::from_str(token_address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid token address: {}", e)
        ))?;
    
    let query = chain_query.chain_query().clone();
    chain_query.runtime().block_on(async move {
        query.token.get_erc20_decimals(token, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get decimals: {}", e)
    ))
}

/// Get ERC20 token name
/// 
/// Args:
///     token_address (str): ERC20 token contract address
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     str: Token name
pub fn get_token_name(chain_query: &PyChainQuery, token_address: &str, block_number: Option<u64>) -> PyResult<String> {
    let token = Address::from_str(token_address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid token address: {}", e)
        ))?;
    
    let query = chain_query.chain_query().clone();
    chain_query.runtime().block_on(async move {
        query.token.get_erc20_name(token, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get token name: {}", e)
    ))
}

/// Get ERC20 token symbol
/// 
/// Args:
///     token_address (str): ERC20 token contract address
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     str: Token symbol
pub fn get_token_symbol(chain_query: &PyChainQuery, token_address: &str, block_number: Option<u64>) -> PyResult<String> {
    let token = Address::from_str(token_address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid token address: {}", e)
        ))?;
    
    let query = chain_query.chain_query().clone();
    chain_query.runtime().block_on(async move {
        query.token.get_erc20_symbol(token, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get token symbol: {}", e)
    ))
}

/// Get ERC20 allowance
/// 
/// Args:
///     token_address (str): ERC20 token contract address
///     owner_address (str): Token owner address
///     spender_address (str): Approved spender address
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     str: Allowance amount as string
pub fn get_allowance(chain_query: &PyChainQuery, token_address: &str, owner_address: &str, spender_address: &str, block_number: Option<u64>) -> PyResult<String> {
    let token = Address::from_str(token_address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid token address: {}", e)
        ))?;
    
    let owner = Address::from_str(owner_address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid owner address: {}", e)
        ))?;
    
    let spender = Address::from_str(spender_address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid spender address: {}", e)
        ))?;
    
    let query = chain_query.chain_query().clone();
    let allowance = chain_query.runtime().block_on(async move {
        query.token.get_allowance(token, owner, spender, block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get allowance: {}", e)
    ))?;
    
    Ok(allowance.to_string())
}

/// Get comprehensive token information
/// 
/// Args:
///     token_address (str): ERC20 token contract address
///     block_number (int, optional): Block number to query at (default: latest)
/// 
/// Returns:
///     dict: Token info with name, symbol, decimals, and total_supply
pub fn get_token_info(chain_query: &PyChainQuery, py: Python, token_address: &str, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
    let token = Address::from_str(token_address.trim_start_matches("0x"))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid token address: {}", e)
        ))?;
    
    let query = chain_query.chain_query().clone();
    
    // Get all token info in parallel
    let (name, symbol, decimals, total_supply) = chain_query.runtime().block_on(async move {
        let name_future = query.token.get_erc20_name(token, block_number);
        let symbol_future = query.token.get_erc20_symbol(token, block_number);
        let decimals_future = query.token.get_erc20_decimals(token, block_number);
        let supply_future = query.get_token_total_supply(token, block_number);
        
        tokio::try_join!(name_future, symbol_future, decimals_future, supply_future)
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get token info: {}", e)
    ))?;
    
    let dict = PyDict::new(py);
    dict.set_item("address", crate::utils::to_checksum_address(&token))?;
    dict.set_item("name", name)?;
    dict.set_item("symbol", symbol)?;
    dict.set_item("decimals", decimals)?;
    dict.set_item("total_supply", total_supply.to_string())?;
    
    Ok(dict.into())
}