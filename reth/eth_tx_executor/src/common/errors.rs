//! Unified error handling for the eth_kartal system
//! 
//! Provides a comprehensive error hierarchy that all modules use,
//! ensuring consistent error handling and reporting.

use ethers::prelude::*;
use std::fmt;
use thiserror::Error;

/// Main error type for the entire eth_kartal system
#[derive(Error, Debug)]
pub enum KartalError {
    /// Alert processing errors
    #[error("Alert error: {0}")]
    Alert(#[from] AlertError),
    
    /// Pool operation errors
    #[error("Pool error: {0}")]
    Pool(#[from] PoolError),
    
    /// Transaction execution errors
    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),
    
    /// Risk management errors
    #[error("Risk error: {0}")]
    Risk(#[from] RiskError),
    
    /// Wallet operation errors
    #[error("Wallet error: {0}")]
    Wallet(#[from] WalletError),
    
    /// Configuration errors
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),
    
    /// Network/RPC errors
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
    
    /// Generic errors
    #[error("{0}")]
    Generic(String),
}

/// Alert processing specific errors
#[derive(Error, Debug)]
pub enum AlertError {
    #[error("Alert expired: received at {received}, deadline was {deadline}")]
    Expired { received: u64, deadline: u64 },
    
    #[error("Invalid alert format: {reason}")]
    InvalidFormat { reason: String },
    
    #[error("Unknown action: {action}")]
    UnknownAction { action: String },
    
    #[error("ZMQ receiver error: {0}")]
    ReceiverError(String),
}

/// Pool operation specific errors
#[derive(Error, Debug)]
pub enum PoolError {
    #[error("Insufficient liquidity: required {required}, available {available}")]
    InsufficientLiquidity { 
        required: U256, 
        available: U256 
    },
    
    #[error("Slippage exceeded: expected {expected}, got {actual}")]
    SlippageExceeded { 
        expected: U256, 
        actual: U256 
    },
    
    #[error("Pool not found: {address}")]
    PoolNotFound { address: Address },
    
    #[error("Invalid pool state: {reason}")]
    InvalidPoolState { reason: String },
    
    #[error("Unsupported protocol: {protocol}")]
    UnsupportedProtocol { protocol: String },
}

/// Transaction execution specific errors
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Transaction failed: {reason}")]
    TransactionFailed { reason: String },
    
    #[error("Nonce too low: expected {expected}, got {actual}")]
    NonceTooLow { expected: U256, actual: U256 },
    
    #[error("Gas price too low: {price} wei")]
    GasPriceTooLow { price: U256 },
    
    #[error("Insufficient balance: need {required}, have {available}")]
    InsufficientBalance { 
        required: U256, 
        available: U256 
    },
    
    #[error("Simulation failed: {reason}")]
    SimulationFailed { reason: String },
    
    #[error("Deadline exceeded")]
    DeadlineExceeded,
    
    #[error("MEV protection failed: {reason}")]
    MevProtectionFailed { reason: String },
}

/// Risk management specific errors
#[derive(Error, Debug)]
pub enum RiskError {
    #[error("Position limit exceeded: {current} > {limit}")]
    PositionLimitExceeded { 
        current: U256, 
        limit: U256 
    },
    
    #[error("Daily loss limit exceeded: {loss} > {limit}")]
    DailyLossLimitExceeded { 
        loss: U256, 
        limit: U256 
    },
    
    #[error("Circuit breaker triggered: {reason}")]
    CircuitBreakerTriggered { reason: String },
    
    #[error("Token blacklisted: {token}")]
    TokenBlacklisted { token: Address },
    
    #[error("Unusual activity detected: {description}")]
    UnusualActivity { description: String },
}

/// Wallet operation specific errors
#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Wallet locked")]
    WalletLocked,
    
    #[error("Invalid password")]
    InvalidPassword,
    
    #[error("Keystore error: {0}")]
    KeystoreError(String),
    
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
}

/// Configuration specific errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },
    
    #[error("File not found: {path}")]
    FileNotFound { path: String },
    
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Network/RPC specific errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("RPC error: {0}")]
    RpcError(String),
    
    #[error("Connection timeout")]
    ConnectionTimeout,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    
    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),
}

/// Convert common external errors to KartalError
impl From<std::io::Error> for KartalError {
    fn from(err: std::io::Error) -> Self {
        KartalError::Generic(format!("IO error: {}", err))
    }
}

impl From<serde_json::Error> for KartalError {
    fn from(err: serde_json::Error) -> Self {
        KartalError::Config(ConfigError::ParseError(err.to_string()))
    }
}

impl From<ContractError<Provider<Http>>> for KartalError {
    fn from(err: ContractError<Provider<Http>>) -> Self {
        KartalError::Network(NetworkError::RpcError(err.to_string()))
    }
}

impl From<ProviderError> for KartalError {
    fn from(err: ProviderError) -> Self {
        KartalError::Network(NetworkError::ProviderError(err))
    }
}

/// Helper trait for adding context to errors
pub trait ErrorContext<T> {
    /// Add context to an error
    fn context(self, msg: &str) -> Result<T, KartalError>;
    
    /// Add formatted context to an error
    fn with_context<F>(self, f: F) -> Result<T, KartalError>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<KartalError>,
{
    fn context(self, msg: &str) -> Result<T, KartalError> {
        self.map_err(|e| {
            let base_error = e.into();
            KartalError::Generic(format!("{}: {}", msg, base_error))
        })
    }
    
    fn with_context<F>(self, f: F) -> Result<T, KartalError>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let base_error = e.into();
            KartalError::Generic(format!("{}: {}", f(), base_error))
        })
    }
}

/// Error severity levels for monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Informational - no action needed
    Info,
    /// Warning - should be investigated
    Warning,
    /// Error - requires intervention
    Error,
    /// Critical - immediate action required
    Critical,
}

impl KartalError {
    /// Get the severity level of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            KartalError::Alert(AlertError::Expired { .. }) => ErrorSeverity::Warning,
            KartalError::Risk(RiskError::CircuitBreakerTriggered { .. }) => ErrorSeverity::Critical,
            KartalError::Execution(ExecutionError::InsufficientBalance { .. }) => ErrorSeverity::Error,
            KartalError::Wallet(WalletError::WalletLocked) => ErrorSeverity::Error,
            KartalError::Network(NetworkError::RateLimitExceeded) => ErrorSeverity::Warning,
            _ => ErrorSeverity::Error,
        }
    }
    
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            KartalError::Network(_) => true,
            KartalError::Execution(ExecutionError::NonceTooLow { .. }) => true,
            KartalError::Alert(AlertError::Expired { .. }) => false,
            KartalError::Risk(_) => false,
            _ => false,
        }
    }
}