//! Bundle builder for Flashbots transactions

use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, instrument};

use super::types::{BundleRequest, BundleConfig};

/// Bundle of transactions to submit atomically
#[derive(Debug, Clone)]
pub struct Bundle {
    /// Signed transactions in order
    pub transactions: Vec<Bytes>,
    /// Target block number
    pub block_number: u64,
    /// Minimum timestamp (optional)
    pub min_timestamp: Option<u64>,
    /// Maximum timestamp (optional)
    pub max_timestamp: Option<u64>,
    /// Transactions that must not revert
    pub reverting_tx_hashes: Vec<H256>,
    /// Tip amount for validator
    pub tip_amount: U256,
}

impl Bundle {
    /// Convert to RPC request format
    pub fn to_request(&self) -> BundleRequest {
        BundleRequest {
            txs: self.transactions
                .iter()
                .map(|tx| format!("0x{}", hex::encode(tx)))
                .collect(),
            block_number: U256::from(self.block_number),
            min_timestamp: self.min_timestamp,
            max_timestamp: self.max_timestamp,
            reverting_tx_hashes: if self.reverting_tx_hashes.is_empty() {
                None
            } else {
                Some(self.reverting_tx_hashes.clone())
            },
        }
    }
    
    /// Calculate bundle hash for tracking
    pub fn hash(&self) -> H256 {
        let mut data = Vec::new();
        for tx in &self.transactions {
            data.extend_from_slice(tx);
        }
        data.extend_from_slice(&self.block_number.to_be_bytes());
        ethers::utils::keccak256(&data).into()
    }
}

/// Builder for constructing Flashbots bundles
pub struct BundleBuilder {
    transactions: Vec<Bytes>,
    block_number: Option<u64>,
    min_timestamp: Option<u64>,
    max_timestamp: Option<u64>,
    reverting_tx_hashes: Vec<H256>,
    tip_percentage: f64,
    min_tip: U256,
    max_tip: U256,
    revert_protection: bool,
}

impl BundleBuilder {
    /// Create new bundle builder
    pub fn new() -> Self {
        Self {
            transactions: Vec::new(),
            block_number: None,
            min_timestamp: None,
            max_timestamp: None,
            reverting_tx_hashes: Vec::new(),
            tip_percentage: 0.01, // 1% default
            min_tip: ethers::utils::parse_ether("0.001").unwrap(), // 0.001 ETH min
            max_tip: ethers::utils::parse_ether("0.1").unwrap(),   // 0.1 ETH max
            revert_protection: true,
        }
    }
    
    /// Create builder from config
    pub fn from_config(config: &BundleConfig) -> Self {
        Self {
            transactions: Vec::new(),
            block_number: None,
            min_timestamp: None,
            max_timestamp: None,
            reverting_tx_hashes: Vec::new(),
            tip_percentage: config.tip_percentage,
            min_tip: config.min_tip,
            max_tip: config.max_tip,
            revert_protection: config.revert_protection,
        }
    }
    
    /// Add signed transaction to bundle
    pub fn add_transaction(mut self, signed_tx: Bytes) -> Self {
        self.transactions.push(signed_tx);
        self
    }
    
    /// Add multiple signed transactions
    pub fn add_transactions(mut self, signed_txs: Vec<Bytes>) -> Self {
        self.transactions.extend(signed_txs);
        self
    }
    
    /// Set target block number
    pub fn block_number(mut self, block: u64) -> Self {
        self.block_number = Some(block);
        self
    }
    
    /// Set minimum timestamp
    pub fn min_timestamp(mut self, timestamp: u64) -> Self {
        self.min_timestamp = Some(timestamp);
        self
    }
    
    /// Set maximum timestamp  
    pub fn max_timestamp(mut self, timestamp: u64) -> Self {
        self.max_timestamp = Some(timestamp);
        self
    }
    
    /// Set time window (current time + window)
    pub fn time_window(mut self, seconds: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.min_timestamp = Some(now);
        self.max_timestamp = Some(now + seconds);
        self
    }
    
    /// Add transaction that must not revert
    pub fn protect_transaction(mut self, tx_hash: H256) -> Self {
        if self.revert_protection {
            self.reverting_tx_hashes.push(tx_hash);
        }
        self
    }
    
    /// Set tip percentage
    pub fn tip_percentage(mut self, percentage: f64) -> Self {
        self.tip_percentage = percentage;
        self
    }
    
    /// Disable revert protection
    pub fn allow_reverts(mut self) -> Self {
        self.revert_protection = false;
        self.reverting_tx_hashes.clear();
        self
    }
    
    /// Build the bundle
    #[instrument(skip(self))]
    pub fn build(self) -> Result<Bundle, Box<dyn std::error::Error>> {
        if self.transactions.is_empty() {
            return Err("Bundle must contain at least one transaction".into());
        }
        
        let block_number = self.block_number
            .ok_or("Block number must be specified")?;
        
        // Calculate tip based on transaction values
        let tip_amount = self.calculate_tip_amount();
        
        debug!(
            "Built bundle with {} transactions for block {}, tip: {}",
            self.transactions.len(),
            block_number,
            ethers::utils::format_ether(tip_amount)
        );
        
        Ok(Bundle {
            transactions: self.transactions,
            block_number,
            min_timestamp: self.min_timestamp,
            max_timestamp: self.max_timestamp,
            reverting_tx_hashes: self.reverting_tx_hashes,
            tip_amount,
        })
    }
    
    /// Calculate appropriate tip amount
    fn calculate_tip_amount(&self) -> U256 {
        // In production, analyze transaction values to determine tip
        // For now, use minimum tip
        self.min_tip
    }
}

/// Helper to extract transaction value from signed transaction
pub fn extract_transaction_value(signed_tx: &Bytes) -> Result<U256, Box<dyn std::error::Error>> {
    // Decode RLP to get transaction value
    // This is simplified - in production would properly decode transaction type
    Ok(U256::zero()) // Placeholder
}

/// Helper to sign a transaction for bundle inclusion
pub async fn sign_for_bundle(
    tx: TypedTransaction,
    signer: &LocalWallet,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    let signature = signer.sign_transaction(&tx).await?;
    let signed = tx.rlp_signed(&signature);
    Ok(signed)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bundle_builder() {
        let bundle = BundleBuilder::new()
            .add_transaction(Bytes::from(vec![1, 2, 3]))
            .block_number(12345)
            .time_window(120) // 2 minutes
            .tip_percentage(0.02) // 2%
            .build()
            .unwrap();
        
        assert_eq!(bundle.transactions.len(), 1);
        assert_eq!(bundle.block_number, 12345);
        assert!(bundle.min_timestamp.is_some());
        assert!(bundle.max_timestamp.is_some());
    }
    
    #[test]
    fn test_bundle_to_request() {
        let bundle = Bundle {
            transactions: vec![Bytes::from(vec![0xaa, 0xbb, 0xcc])],
            block_number: 12345,
            min_timestamp: Some(1234567890),
            max_timestamp: Some(1234567950),
            reverting_tx_hashes: vec![],
            tip_amount: U256::from(1000),
        };
        
        let request = bundle.to_request();
        assert_eq!(request.txs.len(), 1);
        assert_eq!(request.txs[0], "0xaabbcc");
        assert_eq!(request.block_number, U256::from(12345));
    }
}