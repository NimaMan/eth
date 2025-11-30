use alloy_primitives::{Address, B256};
use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use reth_chain_query::BlockTimeConverter;
use std::str::FromStr;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Parse an address string (with or without 0x prefix) into Address
pub fn parse_address(address: &str) -> PyResult<Address> {
    let cleaned = if address.starts_with("0x") || address.starts_with("0X") {
        &address[2..]
    } else {
        address
    };

    Address::from_str(cleaned).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid address '{}': {}",
            address, e
        ))
    })
}

/// Parse a transaction hash (with or without 0x prefix) into B256
pub fn parse_hash(hash: &str) -> PyResult<B256> {
    let cleaned = if hash.starts_with("0x") || hash.starts_with("0X") {
        &hash[2..]
    } else {
        hash
    };

    B256::from_str(cleaned).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid hash '{}': {}", hash, e))
    })
}

/// Parse an ISO8601 timestamp (with optional trailing 'Z') into a UTC datetime
pub fn parse_iso_timestamp(timestamp: &str) -> PyResult<DateTime<Utc>> {
    let trimmed = timestamp.trim();
    let normalized = if trimmed.ends_with('Z') {
        trimmed.replace('Z', "+00:00")
    } else {
        trimmed.to_string()
    };

    DateTime::parse_from_rfc3339(&normalized)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid ISO 8601 timestamp '{}': {}",
                timestamp, e
            ))
        })
}

/// Convert a timestamp to the block number at or before that instant
pub fn timestamp_to_block_floor(
    runtime: &Arc<Runtime>,
    converter: &Arc<BlockTimeConverter>,
    timestamp: DateTime<Utc>,
) -> PyResult<u64> {
    let converter_clone = Arc::clone(converter);
    let mut block = runtime
        .block_on(async move { converter_clone.timestamp_to_block(timestamp).await })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

    if block > 0 {
        let converter_clone = Arc::clone(converter);
        let block_timestamp = runtime
            .block_on(async move { converter_clone.block_to_timestamp(block).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        if block_timestamp > timestamp {
            block -= 1;
        }
    }

    Ok(block)
}

/// Convert a block number into its timestamp using the shared converter
pub fn block_to_timestamp(
    runtime: &Arc<Runtime>,
    converter: &Arc<BlockTimeConverter>,
    block_number: u64,
) -> PyResult<DateTime<Utc>> {
    let converter_clone = Arc::clone(converter);
    runtime
        .block_on(async move { converter_clone.block_to_timestamp(block_number).await })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}
