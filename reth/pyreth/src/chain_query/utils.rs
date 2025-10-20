use alloy_primitives::{Address, B256};
use pyo3::prelude::*;
use std::str::FromStr;

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
