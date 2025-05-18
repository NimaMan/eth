/*
* Transaction Status Types
*
* Types for tracking transaction status, confirmations, and receipts.
*/

use ethers::types::{Address, U256, Bytes, H256, Log, TransactionReceipt as EthersReceipt};
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};

/// Status of a transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxStatus {
    /// Transaction is not yet submitted
    NotSubmitted,
    /// Transaction is waiting to be mined
    Pending,
    /// Transaction is mined but not yet confirmed
    Mined,
    /// Transaction is confirmed
    Confirmed,
    /// Transaction failed
    Failed,
    /// Transaction was dropped from the mempool
    Dropped,
    /// Transaction replaced by another (replaced_by contains the hash)
    Replaced,
    /// The node lost track of the transaction
    Unknown,
}

/// Transaction receipt with additional tracking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxReceipt {
    /// Transaction hash
    pub tx_hash: H256,
    /// Block hash
    pub block_hash: Option<H256>,
    /// Block number
    pub block_number: Option<U256>,
    /// Transaction index in block
    pub transaction_index: Option<U256>,
    /// From address
    pub from: Address,
    /// To address
    pub to: Option<Address>,
    /// Cumulative gas used
    pub cumulative_gas_used: U256,
    /// Gas used
    pub gas_used: Option<U256>,
    /// Contract address created (if any)
    pub contract_address: Option<Address>,
    /// Logs
    pub logs: Vec<Log>,
    /// Status (1 for success, 0 for failure)
    pub status: Option<U256>,
    /// Effective gas price
    pub effective_gas_price: Option<U256>,
    /// When the receipt was last updated
    pub last_updated: Instant,
}

impl From<EthersReceipt> for TxReceipt {
    fn from(receipt: EthersReceipt) -> Self {
        Self {
            tx_hash: receipt.transaction_hash,
            block_hash: receipt.block_hash,
            block_number: receipt.block_number,
            transaction_index: receipt.transaction_index,
            from: receipt.from,
            to: receipt.to,
            cumulative_gas_used: receipt.cumulative_gas_used,
            gas_used: receipt.gas_used,
            contract_address: receipt.contract_address,
            logs: receipt.logs,
            status: receipt.status,
            effective_gas_price: receipt.effective_gas_price,
            last_updated: Instant::now(),
        }
    }
}

/// Confirmation information for a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxConfirmation {
    /// Transaction hash
    pub tx_hash: H256,
    /// Current status
    pub status: TxStatus,
    /// Receipt (if mined)
    pub receipt: Option<TxReceipt>,
    /// Block when the transaction was mined
    pub mined_at_block: Option<u64>,
    /// Current block number
    pub current_block: u64,
    /// Number of confirmations (current_block - mined_at_block)
    pub confirmations: u64,
    /// Replacement transaction hash (if replaced)
    pub replaced_by: Option<H256>,
    /// First submission time
    pub first_submitted_at: Instant,
    /// Last submission time (for resubmissions)
    pub last_submitted_at: Instant,
    /// Number of resubmission attempts
    pub submission_attempts: u32,
    /// Time since first submission
    pub time_since_submission: Duration,
    /// Error message (if any)
    pub error: Option<String>,
}

impl TxConfirmation {
    /// Create a new transaction confirmation for a pending transaction
    pub fn new_pending(tx_hash: H256) -> Self {
        let now = Instant::now();
        Self {
            tx_hash,
            status: TxStatus::Pending,
            receipt: None,
            mined_at_block: None,
            current_block: 0,
            confirmations: 0,
            replaced_by: None,
            first_submitted_at: now,
            last_submitted_at: now,
            submission_attempts: 1,
            time_since_submission: Duration::from_secs(0),
            error: None,
        }
    }
    
    /// Update the status based on a new receipt
    pub fn update_with_receipt(&mut self, receipt: TxReceipt, current_block: u64) -> bool {
        let changed = self.receipt.is_none() || 
            self.mined_at_block.is_none() || 
            self.status == TxStatus::Pending;
            
        self.receipt = Some(receipt.clone());
        
        if let Some(block_number) = receipt.block_number {
            let block_num = block_number.as_u64();
            self.mined_at_block = Some(block_num);
            self.current_block = current_block;
            self.confirmations = current_block.saturating_sub(block_num);
            
            if self.confirmations > 0 {
                self.status = TxStatus::Confirmed;
            } else {
                self.status = TxStatus::Mined;
            }
            
            // Check for failure
            if let Some(status) = receipt.status {
                if status.is_zero() {
                    self.status = TxStatus::Failed;
                    self.error = Some("Transaction execution failed".to_string());
                }
            }
        }
        
        changed
    }
    
    /// Mark a transaction as dropped
    pub fn mark_dropped(&mut self) -> bool {
        if self.status != TxStatus::Dropped {
            self.status = TxStatus::Dropped;
            self.error = Some("Transaction dropped from mempool".to_string());
            true
        } else {
            false
        }
    }
    
    /// Mark a transaction as replaced
    pub fn mark_replaced(&mut self, replacement_hash: H256) -> bool {
        if self.status != TxStatus::Replaced {
            self.status = TxStatus::Replaced;
            self.replaced_by = Some(replacement_hash);
            true
        } else {
            false
        }
    }
    
    /// Record a resubmission attempt
    pub fn record_resubmission(&mut self) {
        self.submission_attempts += 1;
        self.last_submitted_at = Instant::now();
    }
    
    /// Update timing information
    pub fn update_timing(&mut self) {
        self.time_since_submission = self.first_submitted_at.elapsed();
    }
    
    /// Check if the transaction is finalized (confirmed, failed, or replaced)
    pub fn is_finalized(&self) -> bool {
        matches!(self.status, TxStatus::Confirmed | TxStatus::Failed | TxStatus::Replaced)
    }
    
    /// Check if the transaction needs resubmission
    pub fn needs_resubmission(&self, max_pending_time: Duration, max_attempts: u32) -> bool {
        if self.submission_attempts >= max_attempts {
            return false;
        }
        
        if self.is_finalized() {
            return false;
        }
        
        if self.status == TxStatus::Dropped {
            return true;
        }
        
        self.time_since_submission > max_pending_time
    }
} 