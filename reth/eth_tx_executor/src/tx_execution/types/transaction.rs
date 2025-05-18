/*
* Transaction Types
*
* This module defines the core transaction types used throughout the execution system.
*/

use ethers::types::{Address, U256, Bytes, H256};
use serde::{Serialize, Deserialize};

/// Transaction type (Legacy or EIP-1559)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Legacy transaction (pre-EIP-1559)
    Legacy,
    /// EIP-1559 transaction
    Eip1559,
}

/// Parameters for transaction execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionParams {
    /// From address (sender)
    pub from: Address,
    /// To address (recipient, None for contract creation)
    pub to: Option<Address>,
    /// Value in wei
    pub value: U256,
    /// Transaction data/calldata
    pub data: Bytes,
    /// Gas limit
    pub gas_limit: U256,
    /// Transaction type
    pub tx_type: TransactionType,
    
    // Legacy transaction fields
    /// Gas price (for legacy transactions)
    pub gas_price: Option<U256>,
    
    // EIP-1559 transaction fields
    /// Max fee per gas (for EIP-1559 transactions)
    pub max_fee_per_gas: Option<U256>,
    /// Max priority fee per gas (for EIP-1559 transactions)
    pub max_priority_fee_per_gas: Option<U256>,
    
    /// Chain ID
    pub chain_id: u64,
    /// Nonce (if None, will be automatically determined)
    pub nonce: Option<U256>,
}

/// A fully specified transaction ready for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction parameters
    pub params: TransactionParams,
    /// Transaction hash (if known)
    pub hash: Option<H256>,
    /// Whether this transaction should be submitted with high urgency
    pub urgent: bool,
    /// Maximum number of confirmation blocks to wait for
    pub confirmation_blocks: u64,
    /// Maximum number of resubmission attempts
    pub max_retries: u32,
}

impl Transaction {
    /// Create a new transaction with default settings
    pub fn new(params: TransactionParams) -> Self {
        Self {
            params,
            hash: None,
            urgent: false,
            confirmation_blocks: 3,
            max_retries: 3,
        }
    }
    
    /// Create a new urgent transaction with faster confirmation and more retries
    pub fn new_urgent(params: TransactionParams) -> Self {
        Self {
            params,
            hash: None,
            urgent: true,
            confirmation_blocks: 1,
            max_retries: 5,
        }
    }
}
