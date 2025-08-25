/// Block-related chain queries
/// 
/// This module handles all block-related queries including:
/// - Block information (timestamp, gas, base fee)
/// - Block timestamps
/// - Network health indicators

use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::main::PyChainQuery;

/// Get block information
/// 
/// Args:
///     block_number (int, optional): Block number (default: latest)
/// 
/// Returns:
///     dict: Block info with number, timestamp, base_fee, gas_limit, gas_used
pub fn get_block_info(chain_query: &PyChainQuery, py: Python, block_number: Option<u64>) -> PyResult<Py<PyDict>> {
    let query = chain_query.chain_query().clone();
    let info = chain_query.runtime().block_on(async move {
        query.block.get_block_info(block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get block info: {}", e)
    ))?;
    
    let dict = PyDict::new(py);
    dict.set_item("number", info.number)?;
    dict.set_item("timestamp", info.timestamp)?;
    dict.set_item("base_fee_per_gas", info.base_fee_per_gas)?;
    dict.set_item("gas_limit", info.gas_limit)?;
    dict.set_item("gas_used", info.gas_used)?;
    
    // Add network health indicators
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let block_age = current_time.saturating_sub(info.timestamp);
    let blocks_behind = block_age / 12; // Approximate blocks behind network tip
    
    dict.set_item("block_age_seconds", block_age)?;
    dict.set_item("estimated_blocks_behind", blocks_behind)?;
    
    let sync_status = if block_age <= 30 {
        "synced"
    } else if block_age <= 120 {
        "normal_lag"
    } else {
        "lagging"
    };
    dict.set_item("sync_status", sync_status)?;
    
    Ok(dict.into())
}

/// Get block timestamp
/// 
/// Args:
///     block_number (int, optional): Block number (default: latest)
/// 
/// Returns:
///     int: Unix timestamp
pub fn get_block_timestamp(chain_query: &PyChainQuery, block_number: Option<u64>) -> PyResult<u64> {
    let query = chain_query.chain_query().clone();
    chain_query.runtime().block_on(async move {
        query.block.get_block_timestamp(block_number).await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
        format!("Failed to get block timestamp: {}", e)
    ))
}