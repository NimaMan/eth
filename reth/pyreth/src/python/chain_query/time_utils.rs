/// Python bindings for block-time conversion utilities
/// 
/// Provides bidirectional block ↔ time conversion for Python code.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use chrono::{DateTime, Utc};
use std::str::FromStr;

use super::main::PyChainQuery;

/// Convert block number to timestamp
/// 
/// Args:
///     chain_query: ChainQuery instance
///     block_number: Block number to convert
/// 
/// Returns:
///     ISO 8601 timestamp string
#[pyfunction]
pub fn block_to_timestamp(
    chain_query: &PyChainQuery,
    block_number: u64,
) -> PyResult<String> {
    let runtime = chain_query.runtime();
    
    let timestamp = runtime.block_on(async {
        chain_query.chain_query()
            .time_converter
            .block_to_timestamp(block_number)
            .await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    Ok(timestamp.to_rfc3339())
}

/// Convert timestamp to block number
/// 
/// Args:
///     chain_query: ChainQuery instance
///     timestamp: ISO 8601 timestamp string
/// 
/// Returns:
///     Block number
#[pyfunction]
pub fn timestamp_to_block(
    chain_query: &PyChainQuery,
    timestamp: &str,
) -> PyResult<u64> {
    let runtime = chain_query.runtime();
    
    let dt = DateTime::parse_from_rfc3339(timestamp)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid timestamp format: {}", e)
        ))?
        .with_timezone(&Utc);
    
    let block = runtime.block_on(async {
        chain_query.chain_query()
            .time_converter
            .timestamp_to_block(dt)
            .await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    Ok(block)
}

/// Get block range for a time period
/// 
/// Args:
///     chain_query: ChainQuery instance
///     start_timestamp: Start time (ISO 8601)
///     end_timestamp: End time (ISO 8601)
/// 
/// Returns:
///     Tuple of (start_block, end_block)
#[pyfunction]
pub fn get_blocks_for_period(
    chain_query: &PyChainQuery,
    start_timestamp: &str,
    end_timestamp: &str,
) -> PyResult<(u64, u64)> {
    let runtime = chain_query.runtime();
    
    let start_dt = DateTime::parse_from_rfc3339(start_timestamp)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid start timestamp: {}", e)
        ))?
        .with_timezone(&Utc);
    
    let end_dt = DateTime::parse_from_rfc3339(end_timestamp)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid end timestamp: {}", e)
        ))?
        .with_timezone(&Utc);
    
    let (start_block, end_block) = runtime.block_on(async {
        chain_query.chain_query()
            .time_converter
            .get_blocks_for_period(start_dt, end_dt)
            .await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    Ok((start_block, end_block))
}

/// Get blocks for last N periods
/// 
/// Args:
///     chain_query: ChainQuery instance
///     n: Number of periods
///     period_type: Type of period ('hour', 'day', 'week', 'month')
/// 
/// Returns:
///     List of period boundaries with blocks
pub fn get_blocks_for_last_n_periods(
    chain_query: &PyChainQuery,
    py: Python,
    n: usize,
    period_type: &str,
) -> PyResult<Py<PyList>> {
    let runtime = chain_query.runtime();
    
    let period = match period_type {
        "minute" => reth_chain_query::PeriodType::Minute,
        "hour" => reth_chain_query::PeriodType::Hour,
        "4hour" => reth_chain_query::PeriodType::FourHour,
        "day" => reth_chain_query::PeriodType::Day,
        "week" => reth_chain_query::PeriodType::Week,
        "month" => reth_chain_query::PeriodType::Month,
        "quarter" => reth_chain_query::PeriodType::Quarter,
        "year" => reth_chain_query::PeriodType::Year,
        _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid period type: {}", period_type)
        )),
    };
    
    let boundaries = runtime.block_on(async {
        chain_query.chain_query()
            .time_converter
            .get_blocks_for_last_n_periods(n, period)
            .await
    }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    
    let list = PyList::empty(py);
    
    for boundary in boundaries {
        let dict = PyDict::new(py);
        dict.set_item("period_index", boundary.period_index)?;
        dict.set_item("start_time", boundary.start_time.to_rfc3339())?;
        dict.set_item("end_time", boundary.end_time.to_rfc3339())?;
        dict.set_item("start_block", boundary.start_block)?;
        dict.set_item("end_block", boundary.end_block)?;
        list.append(dict)?;
    }
    
    Ok(list.into())
}

/// Estimate timestamp for a block (without database access)
/// 
/// Args:
///     block_number: Block number
/// 
/// Returns:
///     Estimated ISO 8601 timestamp
#[pyfunction]
pub fn estimate_timestamp(block_number: u64) -> String {
    reth_chain_query::time_utils::estimate_timestamp(block_number).to_rfc3339()
}

/// Estimate block number for a timestamp (without database access)
/// 
/// Args:
///     timestamp: ISO 8601 timestamp string
/// 
/// Returns:
///     Estimated block number
#[pyfunction]
pub fn estimate_block_number(timestamp: &str) -> PyResult<u64> {
    let dt = DateTime::parse_from_rfc3339(timestamp)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid timestamp format: {}", e)
        ))?
        .with_timezone(&Utc);
    
    Ok(reth_chain_query::time_utils::estimate_block_number(dt))
}

/// Get number of blocks in a period type
/// 
/// Args:
///     period_type: Type of period ('hour', 'day', 'week', etc.)
/// 
/// Returns:
///     Number of blocks in the period
#[pyfunction]
pub fn blocks_per_period(period_type: &str) -> PyResult<u64> {
    let period = match period_type {
        "block" => reth_chain_query::PeriodType::Block,
        "minute" => reth_chain_query::PeriodType::Minute,
        "hour" => reth_chain_query::PeriodType::Hour,
        "4hour" => reth_chain_query::PeriodType::FourHour,
        "day" => reth_chain_query::PeriodType::Day,
        "week" => reth_chain_query::PeriodType::Week,
        "month" => reth_chain_query::PeriodType::Month,
        "quarter" => reth_chain_query::PeriodType::Quarter,
        "year" => reth_chain_query::PeriodType::Year,
        _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid period type: {}", period_type)
        )),
    };
    
    Ok(period.blocks_per_period())
}