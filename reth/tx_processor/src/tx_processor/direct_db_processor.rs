use eyre::Result;
use std::time::Instant;

// Ethers for compatibility
use ethers_core::types::H256 as EthersH256;
use alloy_primitives::{B256, U256, Address};

// Reth for direct database access using the working pattern from db-access example
// Disabled due to REVM version conflicts - use reth_db_reader approach instead
// use reth_ethereum::{
//     chainspec::ChainSpecBuilder,
//     node::EthereumNode,
//     provider::{
//         providers::ReadOnlyConfig, 
//         TransactionsProvider, ReceiptProvider,
//     },
//     TransactionSigned,
// };

use super::types::ProcessedTransaction;

/// Configuration for the direct database processor
#[derive(Debug, Clone)]
pub struct DirectDbProcessorConfig {
    pub reth_datadir: String,
    pub enable_detailed_analysis: bool,
    pub processing_timeout_ms: u64,
}

impl Default for DirectDbProcessorConfig {
    fn default() -> Self {
        Self {
            reth_datadir: std::env::var("RETH_DATADIR").unwrap_or_else(|_| 
                "/home/nima/.local/share/reth/mainnet".to_string()
            ),
            enable_detailed_analysis: true,
            processing_timeout_ms: 5000,
        }
    }
}

/// Direct database transaction processor using Reth's provider abstractions
pub struct DirectDbProcessor {
    provider_factory: reth_ethereum::ProviderFactory,
    config: DirectDbProcessorConfig,
}

impl DirectDbProcessor {
    /// Create a new direct database processor using Reth's provider pattern
    pub fn new(config: DirectDbProcessorConfig) -> Result<Self> {
        // Instantiate a provider factory for Ethereum mainnet using the provided datadir path
        let spec = ChainSpecBuilder::mainnet().build();
        let provider_factory = EthereumNode::provider_factory_builder()
            .open_read_only(spec.into(), ReadOnlyConfig::from_datadir(&config.reth_datadir))?;
        
        Ok(Self {
            provider_factory,
            config,
        })
    }

    /// Process a transaction using direct database access - returning our standard ProcessedTransaction
    pub fn process_transaction(&self, tx_hash: EthersH256) -> Result<ProcessedTransaction> {
        let total_start = Instant::now();
        
        // Convert ethers H256 to alloy B256
        let tx_hash_b256 = B256::from_slice(tx_hash.as_bytes());
        
        // Direct database access - no network calls using Reth's provider pattern
        let db_start = Instant::now();
        let provider = self.provider_factory.provider()?;
        
        // Get transaction with metadata using Reth's provider API
        let (signed_tx, meta) = provider.transaction_by_hash_with_meta(tx_hash_b256)?
            .ok_or_else(|| eyre::eyre!("Transaction not found: {}", tx_hash))?;
        
        // Get receipt using Reth's provider API
        let receipt = provider.receipt_by_hash(tx_hash_b256)?;
        
        let db_time = db_start.elapsed().as_secs_f64() * 1000.0;
        
        // Recover sender
        let sender = signed_tx.recover_signer()
            .ok_or_else(|| eyre::eyre!("Failed to recover sender"))?;
        
        let total_time = total_start.elapsed().as_secs_f64() * 1000.0;
        
        // Convert to our standard ProcessedTransaction format
        let processed_tx = ProcessedTransaction {
            hash: format!("{:?}", tx_hash_b256),
            block_number: meta.block_number,
            transaction_index: meta.index as u32,
            from_address: format!("{:?}", sender),
            to_address: self.get_to_address(&signed_tx.transaction).map(|addr| format!("{:?}", addr)),
            value: self.get_transaction_value(&signed_tx.transaction).to_string(),
            gas_used: receipt.as_ref().map(|r| r.cumulative_gas_used).unwrap_or(0),
            gas_limit: self.get_gas_limit(&signed_tx.transaction),
            gas_price: self.get_gas_price(&signed_tx.transaction).map(|price| price.to_string()),
            status: receipt.as_ref().map(|r| r.success).unwrap_or(false),
            nonce: self.get_nonce(&signed_tx.transaction),
            input_data: hex::encode(self.get_input_data(&signed_tx.transaction)),
            
            // Timing metrics - this is where we showcase the speed improvement
            fetch_time_ms: db_time, // Direct DB access time
            simulation_time_ms: 0.0, // No simulation yet, just data fetch
            total_time_ms: total_time,
            
            // Analysis results
            transaction_type: self.determine_transaction_type(&signed_tx.transaction),
            is_contract_call: self.is_contract_call(&signed_tx.transaction),
            is_contract_creation: self.is_contract_creation(&signed_tx.transaction),
            has_value_transfer: self.get_transaction_value(&signed_tx.transaction) > U256::ZERO,
            logs_count: receipt.as_ref().map(|r| r.logs.len()).unwrap_or(0),
            
            // Store raw data for potential REVM simulation
            raw_signed_transaction: Some(signed_tx),
            raw_receipt: receipt,
            
            // Empty processed data (can be filled by REVM simulation later)
            internal_transfers: Vec::new(),
            state_changes: std::collections::HashMap::new(),
            processed_logs: Vec::new(),
        };
        
        Ok(processed_tx)
    }

    /// Get the transaction for REVM simulation (if needed)
    pub fn get_transaction_for_simulation(&self, tx_hash: EthersH256) -> Result<TransactionSigned> {
        let tx_hash_b256 = B256::from_slice(tx_hash.as_bytes());
        let provider = self.provider_factory.provider()?;
        
        let (signed_tx, _meta) = provider.transaction_by_hash_with_meta(tx_hash_b256)?
            .ok_or_else(|| eyre::eyre!("Transaction not found: {}", tx_hash))?;
        
        Ok(signed_tx)
    }

    // Helper methods to extract transaction data
    fn get_to_address(&self, tx: &reth_ethereum::Transaction) -> Option<Address> {
        match tx {
            reth_ethereum::Transaction::Legacy(t) => t.to,
            reth_ethereum::Transaction::Eip2930(t) => t.to,
            reth_ethereum::Transaction::Eip1559(t) => t.to,
            reth_ethereum::Transaction::Eip4844(t) => t.to,
            reth_ethereum::Transaction::Eip7702(t) => t.to,
        }
    }

    fn get_transaction_value(&self, tx: &reth_ethereum::Transaction) -> U256 {
        match tx {
            reth_ethereum::Transaction::Legacy(t) => t.value,
            reth_ethereum::Transaction::Eip2930(t) => t.value,
            reth_ethereum::Transaction::Eip1559(t) => t.value,
            reth_ethereum::Transaction::Eip4844(t) => t.value,
            reth_ethereum::Transaction::Eip7702(t) => t.value,
        }
    }

    fn get_gas_limit(&self, tx: &reth_ethereum::Transaction) -> u64 {
        match tx {
            reth_ethereum::Transaction::Legacy(t) => t.gas_limit,
            reth_ethereum::Transaction::Eip2930(t) => t.gas_limit,
            reth_ethereum::Transaction::Eip1559(t) => t.gas_limit,
            reth_ethereum::Transaction::Eip4844(t) => t.gas_limit,
            reth_ethereum::Transaction::Eip7702(t) => t.gas_limit,
        }
    }

    fn get_gas_price(&self, tx: &reth_ethereum::Transaction) -> Option<U256> {
        match tx {
            reth_ethereum::Transaction::Legacy(t) => Some(U256::from(t.gas_price)),
            reth_ethereum::Transaction::Eip2930(t) => Some(U256::from(t.gas_price)),
            reth_ethereum::Transaction::Eip1559(_) => None, // EIP-1559 uses max_fee_per_gas
            reth_ethereum::Transaction::Eip4844(_) => None, // EIP-4844 uses max_fee_per_gas
            reth_ethereum::Transaction::Eip7702(_) => None, // EIP-7702 uses max_fee_per_gas
        }
    }

    fn get_nonce(&self, tx: &reth_ethereum::Transaction) -> u64 {
        match tx {
            reth_ethereum::Transaction::Legacy(t) => t.nonce,
            reth_ethereum::Transaction::Eip2930(t) => t.nonce,
            reth_ethereum::Transaction::Eip1559(t) => t.nonce,
            reth_ethereum::Transaction::Eip4844(t) => t.nonce,
            reth_ethereum::Transaction::Eip7702(t) => t.nonce,
        }
    }

    fn get_input_data(&self, tx: &reth_ethereum::Transaction) -> Vec<u8> {
        match tx {
            reth_ethereum::Transaction::Legacy(t) => t.input.to_vec(),
            reth_ethereum::Transaction::Eip2930(t) => t.input.to_vec(),
            reth_ethereum::Transaction::Eip1559(t) => t.input.to_vec(),
            reth_ethereum::Transaction::Eip4844(t) => t.input.to_vec(),
            reth_ethereum::Transaction::Eip7702(t) => t.input.to_vec(),
        }
    }

    fn determine_transaction_type(&self, tx: &reth_ethereum::Transaction) -> String {
        match tx {
            reth_ethereum::Transaction::Legacy(_) => {
                if self.is_contract_creation(tx) {
                    "Contract Creation".to_string()
                } else if self.is_contract_call(tx) {
                    "Contract Call".to_string()
                } else {
                    "ETH Transfer".to_string()
                }
            },
            reth_ethereum::Transaction::Eip2930(_) => "EIP-2930".to_string(),
            reth_ethereum::Transaction::Eip1559(_) => "EIP-1559".to_string(),
            reth_ethereum::Transaction::Eip4844(_) => "EIP-4844".to_string(),
            reth_ethereum::Transaction::Eip7702(_) => "EIP-7702".to_string(),
        }
    }

    fn is_contract_call(&self, tx: &reth_ethereum::Transaction) -> bool {
        self.get_to_address(tx).is_some() && !self.get_input_data(tx).is_empty()
    }

    fn is_contract_creation(&self, tx: &reth_ethereum::Transaction) -> bool {
        self.get_to_address(tx).is_none()
    }

    /// Get configuration
    pub fn config(&self) -> &DirectDbProcessorConfig {
        &self.config
    }
}