use crate::reth_index::models::{Currency, TradeData};
/// Trades Table
///
/// Tracks aggregated trading activity for address-token pairs.
///
/// Schema:
/// - Key: (Address, TokenAddress, Currency)
/// - Value: TradeData struct
use alloy_primitives::Address;
use eyre::Result;

/// Key for the trades table
#[derive(Debug, Clone)]
pub struct TradeKey {
    pub address: Address,
    pub token: Address,
    pub currency: Currency,
}

/// Trades table interface
pub struct TradesTable;

impl TradesTable {
    /// Table name in MDBX
    pub const TABLE_NAME: &'static str = "trades";

    /// Encode a trade key
    pub fn encode_key(key: &TradeKey) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(41);
        bytes.extend_from_slice(key.address.as_slice());
        bytes.extend_from_slice(key.token.as_slice());
        bytes.push(key.currency.to_byte());
        bytes
    }

    /// Decode key bytes
    pub fn decode_key(bytes: &[u8]) -> Result<TradeKey> {
        if bytes.len() != 41 {
            return Err(eyre::eyre!("Invalid trade key length: {}", bytes.len()));
        }

        let mut address_bytes = [0u8; 20];
        address_bytes.copy_from_slice(&bytes[0..20]);

        let mut token_bytes = [0u8; 20];
        token_bytes.copy_from_slice(&bytes[20..40]);

        Ok(TradeKey {
            address: Address::from(address_bytes),
            token: Address::from(token_bytes),
            currency: Currency::from_byte(bytes[40]),
        })
    }

    /// Encode trade data as value bytes
    pub fn encode_value(data: &TradeData) -> Result<Vec<u8>> {
        // Use bincode or manual serialization
        // For now, use JSON as placeholder
        let json = serde_json::to_vec(data)?;
        Ok(json)
    }

    /// Decode value bytes to trade data
    pub fn decode_value(bytes: &[u8]) -> Result<TradeData> {
        let data = serde_json::from_slice(bytes)?;
        Ok(data)
    }
}
