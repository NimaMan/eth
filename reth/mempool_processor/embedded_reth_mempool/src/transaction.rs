use std::time::Instant;
use reth_transaction_pool::{SubPool, EthPooledTransaction, PoolTransaction};
use serde::{Deserialize, Serialize};
use alloy_consensus::transaction::Transaction;

/// Transaction with embedded timing information for ultra-low latency processing
#[derive(Debug, Clone)]
pub struct EmbeddedTransaction {
    /// The pooled transaction from Reth
    pub pooled_tx: EthPooledTransaction,
    /// When we received this transaction 
    pub received_at: Instant,
    /// Which transaction subpool it belongs to
    pub subpool: SubPool,
}

impl EmbeddedTransaction {
    /// Create new embedded transaction
    pub fn new(pooled_tx: EthPooledTransaction, subpool: SubPool) -> Self {
        Self {
            pooled_tx,
            received_at: Instant::now(),
            subpool,
        }
    }
    
    /// Calculate microseconds since receipt
    pub fn latency_us(&self) -> u64 {
        self.received_at.elapsed().as_micros() as u64
    }
    
    /// Get transaction hash as hex string
    pub fn hash_hex(&self) -> String {
        format!("{:?}", self.pooled_tx.hash())
    }
    
    /// Get the transaction hash as a short identifier
    pub fn hash_short(&self) -> String {
        let hash = self.hash_hex();
        if hash.len() >= 10 {
            hash[..10].to_string()
        } else {
            hash
        }
    }
    
    /// Check if this is a contract interaction (has input data)
    pub fn is_contract_interaction(&self) -> bool {
        !self.pooled_tx.input().is_empty()
    }
    
    /// Get gas limit
    pub fn gas_limit(&self) -> u64 {
        self.pooled_tx.gas_limit()
    }
    
    /// Get nonce
    pub fn nonce(&self) -> u64 {
        self.pooled_tx.nonce()
    }
}

impl From<&reth_transaction_pool::NewTransactionEvent<EthPooledTransaction>> for EmbeddedTransaction {
    fn from(event: &reth_transaction_pool::NewTransactionEvent<EthPooledTransaction>) -> Self {
        Self::new(
            event.transaction.transaction.clone(),
            event.subpool,
        )
    }
}

/// Simplified transaction format for integration with mempool_processor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolTransaction {
    pub hash: String,
    pub hash_short: String,
    pub nonce: u64,
    pub gas_limit: u64,
    pub input_length: usize,
    pub is_contract_interaction: bool,
    pub received_at_us: u64,
    pub subpool: String,
}

impl From<&EmbeddedTransaction> for MempoolTransaction {
    fn from(tx: &EmbeddedTransaction) -> Self {
        Self {
            hash: tx.hash_hex(),
            hash_short: tx.hash_short(),
            nonce: tx.nonce(),
            gas_limit: tx.gas_limit(),
            input_length: tx.pooled_tx.input().len(),
            is_contract_interaction: tx.is_contract_interaction(),
            received_at_us: tx.latency_us(),
            subpool: format!("{:?}", tx.subpool),
        }
    }
}

/// Performance timing breakdown for analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimingBreakdown {
    /// Time from receipt to processing start (microseconds)
    pub receipt_to_processing_us: u64,
    /// Time for processing (signal detection, etc) (microseconds)
    pub processing_duration_us: u64,
    /// Total end-to-end time (microseconds)
    pub total_us: u64,
}

impl TimingBreakdown {
    pub fn new(receipt_time: Instant, processing_start: Instant, processing_end: Instant) -> Self {
        Self {
            receipt_to_processing_us: processing_start.duration_since(receipt_time).as_micros() as u64,
            processing_duration_us: processing_end.duration_since(processing_start).as_micros() as u64,
            total_us: processing_end.duration_since(receipt_time).as_micros() as u64,
        }
    }
}