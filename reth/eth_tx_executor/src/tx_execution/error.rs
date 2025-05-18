/*
* Transaction Execution Error Types
*
* Error handling for transaction execution.
*/

use thiserror::Error;
use std::result;

/// Transaction execution result type
pub type TxResult<T> = result::Result<T, TxError>;

/// Transaction execution errors
#[derive(Error, Debug, Clone)]
pub enum TxError {
    /// Failed to simulate transaction
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),
    
    /// Failed to validate transaction
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    
    /// Failed to estimate gas
    #[error("Gas estimation failed: {0}")]
    GasEstimationFailed(String),
    
    /// Failed to sign transaction
    #[error("Transaction signing failed: {0}")]
    SigningFailed(String),
    
    /// Failed to submit transaction
    #[error("Transaction submission failed: {0}")]
    SubmissionFailed(String),
    
    /// Transaction reverted
    #[error("Transaction reverted: {0}")]
    TransactionReverted(String),
    
    /// Transaction execution failed
    #[error("Transaction execution failed: {0}")]
    ExecutionFailed(String),
    
    /// Failed to monitor transaction
    #[error("Transaction monitoring failed: {0}")]
    MonitoringFailed(String),
    
    /// Transaction dropped from mempool
    #[error("Transaction dropped from mempool: {0}")]
    TransactionDropped(String),
    
    /// Transaction replaced
    #[error("Transaction replaced: {0}")]
    TransactionReplaced(String),
    
    /// Transaction timed out
    #[error("Transaction timed out: {0}")]
    TransactionTimeout(String),
    
    /// Insufficient funds for transaction
    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),
    
    /// Provider error
    #[error("Provider error: {0}")]
    ProviderError(String),
    
    /// Invalid transaction parameters
    #[error("Invalid transaction parameters: {0}")]
    InvalidParams(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}
