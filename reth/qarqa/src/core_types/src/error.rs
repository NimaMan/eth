//! Error types for QARQA analytics

use thiserror::Error;
use alloy_primitives::ruint::ParseError as U256ParseError;
use hex::FromHexError;

/// Main error type for QARQA operations
#[derive(Error, Debug, Clone)]
pub enum QarqaError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Address parsing error: {0}")]
    AddressParsing(String),
    
    #[error("Transaction simulation error: {0}")]
    Simulation(String),
    
    #[error("Network analysis error: {0}")]
    NetworkAnalysis(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Cache error: {0}")]
    Cache(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    #[error("Circuit breaker open: {0}")]
    CircuitBreakerOpen(String),
}

impl From<sqlx::Error> for QarqaError {
    fn from(err: sqlx::Error) -> Self {
        QarqaError::Database(err.to_string())
    }
}

/// Result type for QARQA operations
pub type QarqaResult<T> = Result<T, QarqaError>;

/// Database-specific errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Query failed: {0}")]
    QueryFailed(String),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Pool exhausted")]
    PoolExhausted,
    
    #[error("Timeout")]
    Timeout,
}

/// Simulation-specific errors
#[derive(Error, Debug)]
pub enum SimulationError {
    #[error("Failed to load transaction: {0}")]
    TransactionNotFound(String),
    
    #[error("Failed to load block state: {0}")]
    StateNotFound(String),
    
    #[error("EVM execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Invalid transaction data: {0}")]
    InvalidTransaction(String),
    
    #[error("State provider error: {0}")]
    StateProvider(String),
}

/// Network analysis errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Graph construction failed: {0}")]
    GraphConstruction(String),
    
    #[error("Path finding failed: {0}")]
    PathFinding(String),
    
    #[error("Cycle detection failed: {0}")]
    CycleDetection(String),
    
    #[error("Centrality calculation failed: {0}")]
    CentralityCalculation(String),
    
    #[error("Insufficient data: {0}")]
    InsufficientData(String),
}

/// Convert errors to QarqaError
impl From<DatabaseError> for QarqaError {
    fn from(err: DatabaseError) -> Self {
        QarqaError::Database(err.to_string())
    }
}

impl From<SimulationError> for QarqaError {
    fn from(err: SimulationError) -> Self {
        QarqaError::Simulation(err.to_string())
    }
}

impl From<NetworkError> for QarqaError {
    fn from(err: NetworkError) -> Self {
        QarqaError::NetworkAnalysis(err.to_string())
    }
}

impl From<FromHexError> for QarqaError {
    fn from(err: FromHexError) -> Self {
        QarqaError::AddressParsing(err.to_string())
    }
}

impl From<U256ParseError> for QarqaError {
    fn from(err: U256ParseError) -> Self {
        QarqaError::AddressParsing(err.to_string())
    }
}