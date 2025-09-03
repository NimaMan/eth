/// Address Metrics Table
/// 
/// Stores pre-computed metrics for addresses.
/// 
/// Schema:
/// - Key: Address
/// - Value: AddressMetrics struct

use alloy_primitives::Address;
use eyre::Result;
use crate::reth_index::models::AddressMetrics;

/// Address metrics table interface
pub struct AddressMetricsTable;

impl AddressMetricsTable {
    /// Table name in MDBX
    pub const TABLE_NAME: &'static str = "address_metrics";

    /// Encode an address as key bytes
    pub fn encode_key(address: &Address) -> Vec<u8> {
        address.as_slice().to_vec()
    }

    /// Decode key bytes to address
    pub fn decode_key(bytes: &[u8]) -> Result<Address> {
        if bytes.len() != 20 {
            return Err(eyre::eyre!("Invalid address key length: {}", bytes.len()));
        }
        let mut addr_bytes = [0u8; 20];
        addr_bytes.copy_from_slice(bytes);
        Ok(Address::from(addr_bytes))
    }

    /// Encode metrics as value bytes
    pub fn encode_value(metrics: &AddressMetrics) -> Result<Vec<u8>> {
        let json = serde_json::to_vec(metrics)?;
        Ok(json)
    }

    /// Decode value bytes to metrics
    pub fn decode_value(bytes: &[u8]) -> Result<AddressMetrics> {
        let metrics = serde_json::from_slice(bytes)?;
        Ok(metrics)
    }
}