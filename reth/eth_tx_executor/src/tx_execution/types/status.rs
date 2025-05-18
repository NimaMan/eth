/*
* Transaction Status Types
*
* Types for tracking the status of transactions through their lifecycle.
*/

use ethers::types::{H256, U256, Address, TransactionReceipt as EthersReceipt};
use serde::{Serialize, Deserialize};
use std::time::{Duration, SystemTime};

/// Status of a transaction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxStatus {
    /// Transaction has been created but not yet submitted
    Created,
    
    /// Transaction has been submitted to the network
    Submitted {
        /// Time of submission
        time: SystemTime,
        /// Transaction hash
        hash: H256,
    },
    
    /// Transaction is pending on the network
    Pending {
        /// Time of last status update
        last_updated: SystemTime,
        /// Transaction hash
        hash: H256,
    },
    
    /// Transaction has been mined but not yet confirmed
    Mined {
        /// Time when transaction was mined
        time: SystemTime,
        /// Transaction hash
        hash: H256,
        /// Block hash where transaction was mined
        block_hash: H256,
        /// Block number where transaction was mined
        block_number: U256,
        /// Associated transaction receipt
        receipt: TxReceipt,
    },
    
    /// Transaction has been confirmed
    Confirmed {
        /// Time of confirmation
        time: SystemTime,
        /// Transaction hash
        hash: H256,
        /// Block hash where transaction was mined
        block_hash: H256,
        /// Block number where transaction was mined
        block_number: U256,
        /// Number of confirmations
        confirmations: u64,
        /// Associated transaction receipt
        receipt: TxReceipt,
    },
    
    /// Transaction has failed
    Failed {
        /// Time of failure
        time: SystemTime,
        /// Transaction hash
        hash: H256,
        /// Reason for failure
        reason: String,
        /// Associated transaction receipt (if available)
        receipt: Option<TxReceipt>,
    },
    
    /// Transaction has been replaced (e.g. by a speedup transaction)
    Replaced {
        /// Time of replacement
        time: SystemTime,
        /// Original transaction hash
        original_hash: H256,
        /// Replacement transaction hash
        new_hash: H256,
    },
    
    /// Transaction has timed out (not mined within a reasonable time)
    Timeout {
        /// Time of timeout
        time: SystemTime,
        /// Transaction hash
        hash: H256,
        /// Timeout duration
        timeout: Duration,
    },
}

/// Simplified transaction receipt
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxReceipt {
    /// Transaction hash
    pub transaction_hash: H256,
    
    /// Block hash
    pub block_hash: H256,
    
    /// Block number
    pub block_number: U256,
    
    /// Transaction index within block
    pub transaction_index: U256,
    
    /// Gas used by this transaction
    pub gas_used: U256,
    
    /// Contract address created (if applicable)
    pub contract_address: Option<Address>,
    
    /// Execution success
    pub success: bool,
}

/// Transaction confirmation information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxConfirmation {
    /// Transaction hash
    pub hash: H256,
    
    /// Block hash where transaction was mined
    pub block_hash: H256,
    
    /// Block number where transaction was mined
    pub block_number: U256,
    
    /// Current block number (latest known)
    pub current_block: U256,
    
    /// Number of confirmations
    pub confirmations: u64,
    
    /// Required confirmations (target)
    pub required_confirmations: u64,
    
    /// Whether the transaction has enough confirmations
    pub is_confirmed: bool,
}

impl From<EthersReceipt> for TxReceipt {
    fn from(receipt: EthersReceipt) -> Self {
        Self {
            transaction_hash: receipt.transaction_hash,
            block_hash: receipt.block_hash.unwrap_or_default(),
            block_number: receipt.block_number
                .map(|b| U256::from(b.as_u64()))
                .unwrap_or_default(),
            transaction_index: U256::from(receipt.transaction_index.as_u64()),
            gas_used: receipt.gas_used.unwrap_or_default(),
            contract_address: receipt.contract_address,
            success: receipt.status
                .map(|s| s.as_u64() == 1)
                .unwrap_or(false),
        }
    }
} 