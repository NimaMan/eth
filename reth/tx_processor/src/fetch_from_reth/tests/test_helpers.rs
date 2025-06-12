//! Test helpers and utilities for fetch_from_reth tests

use crate::fetch_from_reth::{
    provider::{RethDataProvider, TransactionData},
    error::{FetchError, FetchResult},
};
use alloy_primitives::{Address, B256, U256, Bytes};
use std::collections::HashMap;
use std::str::FromStr;

/// Mock provider for unit testing
#[derive(Debug, Clone)]
pub struct MockRethProvider {
    transactions: HashMap<B256, TransactionData>,
    error_on: Option<B256>,
    latest_block: u64,
}

impl MockRethProvider {
    /// Create a new mock provider
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
            error_on: None,
            latest_block: 18_000_000,
        }
    }
    
    /// Add a transaction to the mock provider
    pub fn with_transaction(mut self, tx_hash: &str, data: TransactionData) -> Self {
        let hash = B256::from_str(tx_hash).expect("Valid transaction hash");
        self.transactions.insert(hash, data);
        self
    }
    
    /// Configure provider to return error for specific hash
    pub fn with_error_on(mut self, tx_hash: &str) -> Self {
        let hash = B256::from_str(tx_hash).expect("Valid transaction hash");
        self.error_on = Some(hash);
        self
    }
    
    /// Set latest block number
    pub fn with_latest_block(mut self, block_number: u64) -> Self {
        self.latest_block = block_number;
        self
    }
}

impl RethDataProvider for MockRethProvider {
    fn fetch_transaction(&self, tx_hash: B256) -> FetchResult<TransactionData> {
        if let Some(error_hash) = &self.error_on {
            if tx_hash == *error_hash {
                return Err(FetchError::DatabaseError("Mock error".to_string()));
            }
        }
        
        self.transactions
            .get(&tx_hash)
            .cloned()
            .ok_or_else(|| FetchError::transaction_not_found(&format!("{:x}", tx_hash)))
    }
    
    fn fetch_batch(&self, tx_hashes: &[B256]) -> FetchResult<Vec<TransactionData>> {
        let mut results = Vec::new();
        
        for &tx_hash in tx_hashes {
            if let Ok(tx_data) = self.fetch_transaction(tx_hash) {
                results.push(tx_data);
            }
            // Silently skip missing transactions in batch mode
        }
        
        Ok(results)
    }
    
    fn transaction_exists(&self, tx_hash: B256) -> FetchResult<bool> {
        if let Some(error_hash) = &self.error_on {
            if tx_hash == *error_hash {
                return Err(FetchError::DatabaseError("Mock error".to_string()));
            }
        }
        
        Ok(self.transactions.contains_key(&tx_hash))
    }
    
    fn latest_block_number(&self) -> FetchResult<u64> {
        Ok(self.latest_block)
    }
    
    fn fetch_transaction_by_block_and_index(&self, _block_number: u64, _tx_index: u64) -> FetchResult<TransactionData> {
        // For simplicity, just return the first transaction in our mock data
        self.transactions
            .values()
            .next()
            .cloned()
            .ok_or_else(|| FetchError::NotFound("No transactions in mock data".to_string()))
    }
}

/// Create test transaction data
pub fn create_test_transaction_data(hash_suffix: u8) -> TransactionData {
    let mut hash_bytes = [0u8; 32];
    hash_bytes[31] = hash_suffix;
    let hash = B256::from(hash_bytes);
    
    TransactionData {
        hash,
        from: Address::from_str("0x32Be343B94f860124dC4fEe278FDCBD38C102D88").unwrap(),
        to: Some(Address::from_str("0x53b04999c1FF2d77fcdde98935BB936A67209E4C").unwrap()),
        value: U256::from(500000000000000000u64), // 0.5 ETH
        gas_limit: 21000,
        gas_used: 21000,
        gas_price: U256::from(20000000000u64), // 20 gwei
        nonce: hash_suffix as u64,
        block_number: 46147 + hash_suffix as u64,
        block_hash: B256::from([hash_suffix; 32]),
        transaction_index: hash_suffix as u64,
        input: Bytes::new(),
        receipt_status: true,
        contractaddress: None,
        logs: vec![],
    }
}

/// Create contract creation transaction data
pub fn create_contract_creation_data() -> TransactionData {
    let hash = B256::from_str("0x7b0a47d3b0234280b6c9213c5bbff44c8b6001bea7770b3950280f91410532d6").unwrap();
    
    TransactionData {
        hash,
        from: Address::from_str("0x32Be343B94f860124dC4fEe278FDCBD38C102D88").unwrap(),
        to: None, // Contract creation
        value: U256::ZERO,
        gas_limit: 500000,
        gas_used: 350000,
        gas_price: U256::from(20000000000u64),
        nonce: 42,
        block_number: 46148,
        block_hash: B256::from([0xab; 32]),
        transaction_index: 0,
        input: Bytes::from_static(&[0x60, 0x80, 0x60, 0x40]), // Sample bytecode
        receipt_status: true,
        contractaddress: Some(Address::from_str("0x1234567890abcdef1234567890abcdef12345678").unwrap()),
        logs: vec![],
    }
}

/// Create failed transaction data
pub fn create_failed_transaction_data() -> TransactionData {
    let hash = B256::from_str("0x7c90a0b4f5c0e1a94ffd22330e15e18f7da77a1155ad00f7f5d1a1c4a8e36a4a").unwrap();
    
    TransactionData {
        hash,
        from: Address::from_str("0x32Be343B94f860124dC4fEe278FDCBD38C102D88").unwrap(),
        to: Some(Address::from_str("0x53b04999c1FF2d77fcdde98935BB936A67209E4C").unwrap()),
        value: U256::from(1000000000000000000u64), // 1 ETH
        gas_limit: 21000,
        gas_used: 21000, // All gas consumed on failure
        gas_price: U256::from(20000000000u64),
        nonce: 1,
        block_number: 46149,
        block_hash: B256::from([0xcd; 32]),
        transaction_index: 5,
        input: Bytes::new(),
        receipt_status: false, // Failed transaction
        contractaddress: None,
        logs: vec![],
    }
}

/// Well-known mainnet transaction hashes for testing
pub const ETH_TRANSFER_TX: &str = "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060";
pub const ERC20_TRANSFER_TX: &str = "0x2d8ae95d9410c1cefa65c48aa9c2b793eff1e8057d9bf35a2f8e81d9e7b4e5fb";
pub const UNISWAP_V2_SWAP_TX: &str = "0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006";
pub const FAILED_TX: &str = "0x7c90a0b4f5c0e1a94ffd22330e15e18f7da77a1155ad00f7f5d1a1c4a8e36a4a";
pub const CONTRACT_CREATION_TX: &str = "0x7b0a47d3b0234280b6c9213c5bbff44c8b6001bea7770b3950280f91410532d6";

/// Create a configured mock provider with test data
pub fn create_mock_provider_with_test_data() -> MockRethProvider {
    // Create transaction data with the correct hashes
    let eth_transfer_data = {
        let mut data = create_test_transaction_data(1);
        data.hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        data
    };
    
    let erc20_transfer_data = {
        let mut data = create_test_transaction_data(2);
        data.hash = B256::from_str(ERC20_TRANSFER_TX).unwrap();
        data
    };
    
    let uniswap_swap_data = {
        let mut data = create_test_transaction_data(3);
        data.hash = B256::from_str(UNISWAP_V2_SWAP_TX).unwrap();
        data
    };
    
    MockRethProvider::new()
        .with_transaction(ETH_TRANSFER_TX, eth_transfer_data)
        .with_transaction(ERC20_TRANSFER_TX, erc20_transfer_data)
        .with_transaction(UNISWAP_V2_SWAP_TX, uniswap_swap_data)
        .with_transaction(CONTRACT_CREATION_TX, create_contract_creation_data())
        .with_transaction(FAILED_TX, create_failed_transaction_data())
        .with_latest_block(18_500_000)
}

/// Measure execution time of a function
pub fn measure_time<F, R>(operation: F) -> (R, std::time::Duration)
where
    F: FnOnce() -> R,
{
    let start = std::time::Instant::now();
    let result = operation();
    let duration = start.elapsed();
    (result, duration)
}

/// Assert that duration is within expected bounds
pub fn assert_performance(duration: std::time::Duration, max_ms: u64, operation: &str) {
    let actual_ms = duration.as_millis() as u64;
    assert!(
        actual_ms <= max_ms,
        "Performance regression: {} took {}ms, expected <{}ms",
        operation,
        actual_ms,
        max_ms
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mock_provider_creation() {
        let provider = MockRethProvider::new();
        assert_eq!(provider.latest_block_number().unwrap(), 18_000_000);
    }
    
    #[test]
    fn test_mock_provider_with_data() {
        let provider = create_mock_provider_with_test_data();
        
        let hash = B256::from_str(ETH_TRANSFER_TX).unwrap();
        let tx_data = provider.fetch_transaction(hash).unwrap();
        
        assert_eq!(tx_data.hash, hash);
        assert!(tx_data.receipt_status);
    }
    
    #[test]
    fn test_test_data_creation() {
        let tx_data = create_test_transaction_data(1);
        assert_eq!(tx_data.nonce, 1);
        assert_eq!(tx_data.block_number, 46148);
        assert!(tx_data.receipt_status);
    }
    
    #[test]
    fn test_contract_creation_data() {
        let tx_data = create_contract_creation_data();
        assert!(tx_data.to.is_none());
        assert!(tx_data.contractaddress.is_some());
        assert!(!tx_data.input.is_empty());
    }
    
    #[test]
    fn test_failed_transaction_data() {
        let tx_data = create_failed_transaction_data();
        assert!(!tx_data.receipt_status);
        assert_eq!(tx_data.gas_used, tx_data.gas_limit); // All gas consumed
    }
    
    #[test]
    fn test_measure_time() {
        let (result, duration) = measure_time(|| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            42
        });
        
        assert_eq!(result, 42);
        assert!(duration >= std::time::Duration::from_millis(9)); // Allow some variance
    }
}