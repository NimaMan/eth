use crate::reth_index::models::PoolData;
/// Pools Table
///
/// Stores DEX pool information.
///
/// Schema:
/// - Key: PoolAddress
/// - Value: PoolData struct
use alloy_primitives::Address;
use eyre::Result;

/// Pools table interface
pub struct PoolsTable;

impl PoolsTable {
    /// Table name in MDBX
    pub const TABLE_NAME: &'static str = "pools";

    /// Encode a pool address as key bytes
    pub fn encode_key(pool: &Address) -> Vec<u8> {
        pool.as_slice().to_vec()
    }

    /// Decode key bytes to pool address
    pub fn decode_key(bytes: &[u8]) -> Result<Address> {
        if bytes.len() != 20 {
            return Err(eyre::eyre!("Invalid pool key length: {}", bytes.len()));
        }
        let mut addr_bytes = [0u8; 20];
        addr_bytes.copy_from_slice(bytes);
        Ok(Address::from(addr_bytes))
    }

    /// Encode pool data as value bytes
    pub fn encode_value(data: &PoolData) -> Result<Vec<u8>> {
        let json = serde_json::to_vec(data)?;
        Ok(json)
    }

    /// Decode value bytes to pool data
    pub fn decode_value(bytes: &[u8]) -> Result<PoolData> {
        let data = serde_json::from_slice(bytes)?;
        Ok(data)
    }
}
