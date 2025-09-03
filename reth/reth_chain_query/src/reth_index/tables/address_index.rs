/// Address to Transaction Index Table
/// 
/// This table provides the essential reverse index that reth doesn't have:
/// mapping from address to all transactions that address participated in.
/// 
/// Schema:
/// - Key: Address (20 bytes)
/// - Value: Vec<TxNumber> (list of u64 transaction numbers)
/// 
/// Integration with Reth:
/// - TxNumber comes from reth's TransactionHashNumbers table (TxHash → TxNumber)
/// - Block ranges use reth's BlockBodyIndices table (BlockNumber → {first_tx_num, tx_count})
/// - Transaction details fetched from reth's Transactions table (TxNumber → TransactionSigned)

use alloy_primitives::Address;
use eyre::Result;
use serde::{Deserialize, Serialize};

/// Transaction number type (matches reth's TxNumber)
pub type TxNumber = u64;

/// List of transaction numbers for an address
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransactionList {
    /// Transaction numbers in chronological order
    pub transactions: Vec<TxNumber>,
}

impl TransactionList {
    /// Create a new empty list
    pub fn new() -> Self {
        Self {
            transactions: Vec::new(),
        }
    }

    /// Add a transaction to the list
    pub fn add_transaction(&mut self, tx_num: TxNumber) {
        self.transactions.push(tx_num);
    }

    /// Get the number of transactions
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Get transactions in a block range
    /// 
    /// NOTE: This requires querying reth's BlockBodyIndices table to get TxNumber ranges
    /// Example implementation:
    /// ```ignore
    /// let start_indices = reth_db.get::<BlockBodyIndices>(start_block)?;
    /// let end_indices = reth_db.get::<BlockBodyIndices>(end_block)?;
    /// 
    /// self.transactions.iter()
    ///     .filter(|tx| **tx >= start_indices.first_tx_num && 
    ///                  **tx <= end_indices.last_tx_num())
    ///     .cloned()
    ///     .collect()
    /// ```
    pub fn in_range(&self, start_block: u64, end_block: u64) -> Vec<TxNumber> {
        // TODO: Requires reth database connection
        // For now, return all transactions
        self.transactions.clone()
    }

    /// Get the most recent N transactions
    pub fn latest(&self, count: usize) -> Vec<TxNumber> {
        let len = self.transactions.len();
        if count >= len {
            self.transactions.clone()
        } else {
            self.transactions[len - count..].to_vec()
        }
    }
}

/// Address index table interface
pub struct AddressIndex;

impl AddressIndex {
    /// Table name in MDBX
    pub const TABLE_NAME: &'static str = "address_to_txs";

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

    /// Encode transaction list as value bytes
    pub fn encode_value(list: &TransactionList) -> Result<Vec<u8>> {
        // Use bincode for efficient binary encoding
        // Each TxNumber is 8 bytes, plus length prefix
        let mut bytes = Vec::with_capacity(8 + list.transactions.len() * 8);
        
        // Write count as u64
        bytes.extend_from_slice(&(list.transactions.len() as u64).to_le_bytes());
        
        // Write each transaction number
        for tx_num in &list.transactions {
            bytes.extend_from_slice(&tx_num.to_le_bytes());
        }
        
        Ok(bytes)
    }

    /// Decode value bytes to transaction list
    pub fn decode_value(bytes: &[u8]) -> Result<TransactionList> {
        if bytes.len() < 8 {
            return Err(eyre::eyre!("Invalid value: too short"));
        }
        
        // Read count
        let count = u64::from_le_bytes(bytes[0..8].try_into()?) as usize;
        
        // Verify remaining bytes match expected size
        let expected_size = 8 + count * 8;
        if bytes.len() != expected_size {
            return Err(eyre::eyre!(
                "Invalid value size: expected {}, got {}",
                expected_size,
                bytes.len()
            ));
        }
        
        // Read transaction numbers
        let mut transactions = Vec::with_capacity(count);
        for i in 0..count {
            let start = 8 + i * 8;
            let tx_num = u64::from_le_bytes(bytes[start..start + 8].try_into()?);
            transactions.push(tx_num);
        }
        
        Ok(TransactionList { transactions })
    }

    /// Create a composite key for range queries
    /// Format: address || block_number
    pub fn encode_range_key(address: &Address, block: u64) -> Vec<u8> {
        let mut key = Vec::with_capacity(28);
        key.extend_from_slice(address.as_slice());
        key.extend_from_slice(&block.to_be_bytes()); // Big-endian for proper ordering
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_key() {
        let address = Address::repeat_byte(0x42);
        let encoded = AddressIndex::encode_key(&address);
        assert_eq!(encoded.len(), 20);
        
        let decoded = AddressIndex::decode_key(&encoded).unwrap();
        assert_eq!(decoded, address);
    }

    #[test]
    fn test_encode_decode_value() {
        let mut list = TransactionList::new();
        list.add_transaction(1000);
        list.add_transaction(2000);
        list.add_transaction(3000);
        
        let encoded = AddressIndex::encode_value(&list).unwrap();
        let decoded = AddressIndex::decode_value(&encoded).unwrap();
        
        assert_eq!(decoded.transactions, vec![1000, 2000, 3000]);
    }

    #[test]
    fn test_latest_transactions() {
        let mut list = TransactionList::new();
        for i in 0..10 {
            list.add_transaction(i);
        }
        
        let latest = list.latest(3);
        assert_eq!(latest, vec![7, 8, 9]);
        
        let all = list.latest(20);
        assert_eq!(all.len(), 10);
    }
}