/// Transaction converter for Direct Reth Simulator
/// Converts mempool FullTransaction to Reth TransactionSigned format
use eyre::{Result, eyre};
use reth_primitives::TransactionSigned;
use alloy_rlp::Decodable;

// Import mempool transaction types
use crate::mempool_fetcher::FullTransaction;

/// Convert mempool FullTransaction to Reth TransactionSigned
/// This uses the raw transaction bytes from the JSON data
pub fn mempool_tx_to_reth_signed(
    tx: &FullTransaction,
) -> Result<TransactionSigned> {
    // Extract raw transaction bytes from JSON
    let raw = tx.tx_data.get("raw")
        .and_then(|v| v.as_str())
        .ok_or_else(|| eyre!("No raw transaction data in tx_data"))?;
    
    // Remove 0x prefix if present
    let hex_str = raw.strip_prefix("0x").unwrap_or(raw);
    
    // Decode hex string to bytes
    let raw_bytes = hex::decode(hex_str)
        .map_err(|e| eyre!("Failed to decode hex: {}", e))?;
    
    // Decode the transaction using RLP
    // This is the standard way Reth decodes transactions
    let signed_tx = TransactionSigned::decode(&mut raw_bytes.as_slice())
        .map_err(|e| eyre!("Failed to decode transaction: {}", e))?;
    
    Ok(signed_tx)
}

/// Extract transaction metadata for performance tracking
pub struct TransactionMetadata {
    pub network_latency_us: Option<u64>,
    pub detection_time_us: u64,
}

impl TransactionMetadata {
    pub fn from_full_transaction(tx: &FullTransaction) -> Self {
        Self {
            // Convert nanoseconds to microseconds
            network_latency_us: Some(tx.latency_ns / 1000),
            detection_time_us: tx.latency_ns / 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hex_decoding() {
        let with_prefix = "0x1234abcd";
        let without_prefix = "1234abcd";
        
        let stripped1 = with_prefix.strip_prefix("0x").unwrap_or(with_prefix);
        let stripped2 = without_prefix.strip_prefix("0x").unwrap_or(without_prefix);
        
        assert_eq!(stripped1, "1234abcd");
        assert_eq!(stripped2, "1234abcd");
        
        let bytes1 = hex::decode(stripped1).unwrap();
        let bytes2 = hex::decode(stripped2).unwrap();
        
        assert_eq!(bytes1, bytes2);
        assert_eq!(bytes1, vec![0x12, 0x34, 0xab, 0xcd]);
    }
}