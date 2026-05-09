use crate::reth_index::models::TokenMetadata;
/// Tokens Table
///
/// Stores token metadata.
///
/// Schema:
/// - Key: TokenAddress
/// - Value: TokenMetadata struct
use alloy_primitives::Address;
use eyre::Result;

/// Tokens table interface
pub struct TokensTable;

impl TokensTable {
    /// Table name in MDBX
    pub const TABLE_NAME: &'static str = "tokens";

    /// Encode a token address as key bytes
    pub fn encode_key(token: &Address) -> Vec<u8> {
        token.as_slice().to_vec()
    }

    /// Decode key bytes to token address
    pub fn decode_key(bytes: &[u8]) -> Result<Address> {
        if bytes.len() != 20 {
            return Err(eyre::eyre!("Invalid token key length: {}", bytes.len()));
        }
        let mut addr_bytes = [0u8; 20];
        addr_bytes.copy_from_slice(bytes);
        Ok(Address::from(addr_bytes))
    }

    /// Encode token metadata as value bytes
    pub fn encode_value(metadata: &TokenMetadata) -> Result<Vec<u8>> {
        let json = serde_json::to_vec(metadata)?;
        Ok(json)
    }

    /// Decode value bytes to token metadata
    pub fn decode_value(bytes: &[u8]) -> Result<TokenMetadata> {
        let metadata = serde_json::from_slice(bytes)?;
        Ok(metadata)
    }
}
