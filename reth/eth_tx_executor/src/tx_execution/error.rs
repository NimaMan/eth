/*
* Transaction Execution Error Types
*
* This module defines error types for the transaction execution system.
*/

use thiserror::Error;
use ethers::types::{H256, U256};
use std::time::Duration;

#[derive(Debug, Error)]
pub enum TxExecutionError {
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),
    
    #[error("Transaction submission failed: {0}")]
    SubmissionFailed(String),
    
    #[error("Transaction {0} failed: {1}")]
    TransactionFailed(H256, String),
    
    #[error("Transaction {0} timed out after {1:?}")]
    TransactionTimeout(H256, Duration),
    
    #[error("Transaction {0} dropped from mempool")]
    TransactionDropped(H256),
    
    #[error("Transaction {0} replaced by {1}")]
    TransactionReplaced(H256, H256),
    
    #[error("Insufficient funds: required {0}, available {1}")]
    InsufficientFunds(U256, U256),
    
    #[error("Nonce error: expected {0}, got {1}")]
    NonceError(U256, U256),
    
    #[error("Gas estimation failed: {0}")]
    GasEstimationFailed(String),
    
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("Provider error: {0}")]
    ProviderError(String),
    
    #[error("Wallet error: {0}")]
    WalletError(String),
    
    #[error("Circuit breaker triggered: {0}")]
    CircuitBreaker(String),
    
    #[error("Transaction monitoring error: {0}")]
    MonitoringError(String),
    
    #[error("Maximum retry attempts ({0}) exceeded")]
    MaxRetryExceeded(u32),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<ethers::providers::ProviderError> for TxExecutionError {
    fn from(error: ethers::providers::ProviderError) -> Self {
        TxExecutionError::ProviderError(error.to_string())
    }
}

impl From<ethers::signers::WalletError> for TxExecutionError {
    fn from(error: ethers::signers::WalletError) -> Self {
        TxExecutionError::WalletError(error.to_string())
    }
}

/// Result type for transaction execution operations
pub type TxResult<T> = Result<T, TxExecutionError>; 