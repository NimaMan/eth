//! Error types for the fetch_from_reth module
//!
//! This module defines comprehensive error handling for all operations
//! related to fetching data from Reth's MDBX database.

use std::fmt;

/// Result type alias for fetch operations
pub type FetchResult<T> = Result<T, FetchError>;

/// Comprehensive error type for fetch_from_reth operations
#[derive(Debug, Clone)]
pub enum FetchError {
    /// Transaction, block, or other entity not found in database
    NotFound(String),
    
    /// Database connection or access error
    DatabaseError(String),
    
    /// Configuration error (invalid paths, permissions, etc.)
    ConfigError(String),
    
    /// Cache operation error
    CacheError(String),
    
    /// Data parsing or conversion error  
    ParseError(String),
    
    /// Provider factory creation error
    ProviderError(String),
    
    /// Static file access error
    StaticFileError(String),
    
    /// MDBX-specific database errors
    MdbxError(String),
    
    /// I/O errors (file access, permissions, etc.)
    IoError(String),
    
    /// Operation timeout
    TimeoutError(String),
    
    /// Invalid input parameters
    InvalidInput(String),
    
    /// Internal consistency error
    ConsistencyError(String),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::NotFound(msg) => write!(f, "Not found: {}", msg),
            FetchError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            FetchError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            FetchError::CacheError(msg) => write!(f, "Cache error: {}", msg),
            FetchError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            FetchError::ProviderError(msg) => write!(f, "Provider error: {}", msg),
            FetchError::StaticFileError(msg) => write!(f, "Static file error: {}", msg),
            FetchError::MdbxError(msg) => write!(f, "MDBX error: {}", msg),
            FetchError::IoError(msg) => write!(f, "I/O error: {}", msg),
            FetchError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            FetchError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            FetchError::ConsistencyError(msg) => write!(f, "Consistency error: {}", msg),
        }
    }
}

impl std::error::Error for FetchError {}

// Conversions from common error types
impl From<std::io::Error> for FetchError {
    fn from(err: std::io::Error) -> Self {
        FetchError::IoError(format!("I/O error: {}", err))
    }
}

impl From<eyre::Error> for FetchError {
    fn from(err: eyre::Error) -> Self {
        FetchError::DatabaseError(format!("Database error: {}", err))
    }
}

// Helper functions for creating specific error types
impl FetchError {
    /// Create a NotFound error for a transaction hash
    pub fn transaction_not_found(tx_hash: &str) -> Self {
        FetchError::NotFound(format!("Transaction {} not found in database", tx_hash))
    }
    
    /// Create a NotFound error for a block
    pub fn block_not_found(block_ref: &str) -> Self {
        FetchError::NotFound(format!("Block {} not found in database", block_ref))
    }
    
    /// Create a NotFound error for a receipt
    pub fn receipt_not_found(tx_hash: &str) -> Self {
        FetchError::NotFound(format!("Receipt for transaction {} not found", tx_hash))
    }
    
    /// Create a DatabaseError for provider issues
    pub fn provider_creation_failed(reason: &str) -> Self {
        FetchError::ProviderError(format!("Failed to create provider: {}", reason))
    }
    
    /// Create a ConfigError for invalid database path
    pub fn invalid_database_path(path: &str) -> Self {
        FetchError::ConfigError(format!("Invalid database path: {}", path))
    }
    
    /// Create a DatabaseError for MDBX issues
    pub fn mdbx_error(operation: &str, error: &str) -> Self {
        FetchError::MdbxError(format!("MDBX error during {}: {}", operation, error))
    }
    
    /// Create a TimeoutError for operations
    pub fn operation_timeout(operation: &str, duration_ms: u64) -> Self {
        FetchError::TimeoutError(format!("Operation '{}' timed out after {}ms", operation, duration_ms))
    }
    
    /// Create a ConsistencyError for data integrity issues
    pub fn data_inconsistency(description: &str) -> Self {
        FetchError::ConsistencyError(format!("Data consistency error: {}", description))
    }
    
    /// Create an InvalidInput error for bad parameters
    pub fn invalid_parameter(param_name: &str, reason: &str) -> Self {
        FetchError::InvalidInput(format!("Invalid parameter '{}': {}", param_name, reason))
    }
}

/// Error classification for error handling strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Temporary errors that might succeed on retry
    Temporary,
    /// Permanent errors that will not succeed on retry  
    Permanent,
    /// User errors (bad input, configuration, etc.)
    User,
    /// System errors (I/O, permissions, etc.)
    System,
}

impl FetchError {
    /// Classify error for handling strategy
    pub fn category(&self) -> ErrorCategory {
        match self {
            // Temporary errors - might work on retry
            FetchError::TimeoutError(_) => ErrorCategory::Temporary,
            FetchError::MdbxError(_) => ErrorCategory::Temporary,
            
            // Permanent errors - won't work on retry
            FetchError::NotFound(_) => ErrorCategory::Permanent,
            FetchError::ParseError(_) => ErrorCategory::Permanent,
            FetchError::ConsistencyError(_) => ErrorCategory::Permanent,
            
            // User errors - need user intervention
            FetchError::ConfigError(_) => ErrorCategory::User,
            FetchError::InvalidInput(_) => ErrorCategory::User,
            
            // System errors - need system intervention
            FetchError::DatabaseError(_) => ErrorCategory::System,
            FetchError::ProviderError(_) => ErrorCategory::System,
            FetchError::StaticFileError(_) => ErrorCategory::System,
            FetchError::IoError(_) => ErrorCategory::System,
            FetchError::CacheError(_) => ErrorCategory::System,
        }
    }
    
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self.category(), ErrorCategory::Temporary)
    }
    
    /// Check if error indicates missing data
    pub fn is_not_found(&self) -> bool {
        matches!(self, FetchError::NotFound(_))
    }
    
    /// Check if error is due to user input
    pub fn is_user_error(&self) -> bool {
        matches!(self.category(), ErrorCategory::User)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let error = FetchError::transaction_not_found("0x1234");
        assert!(error.to_string().contains("Transaction 0x1234 not found"));
    }
    
    #[test]
    fn test_error_categories() {
        assert_eq!(FetchError::NotFound("test".to_string()).category(), ErrorCategory::Permanent);
        assert_eq!(FetchError::TimeoutError("test".to_string()).category(), ErrorCategory::Temporary);
        assert_eq!(FetchError::ConfigError("test".to_string()).category(), ErrorCategory::User);
        assert_eq!(FetchError::DatabaseError("test".to_string()).category(), ErrorCategory::System);
    }
    
    #[test]
    fn test_error_retryable() {
        assert!(FetchError::TimeoutError("test".to_string()).is_retryable());
        assert!(!FetchError::NotFound("test".to_string()).is_retryable());
    }
    
    #[test]
    fn test_helper_functions() {
        let tx_hash = "0x1234567890abcdef";
        let error = FetchError::transaction_not_found(tx_hash);
        assert!(error.is_not_found());
        assert!(!error.is_retryable());
        
        let error = FetchError::invalid_parameter("block_number", "must be positive");
        assert!(error.is_user_error());
    }
}