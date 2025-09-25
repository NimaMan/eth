use alloy_primitives::Address;
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
