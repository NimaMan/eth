//! Crate-Specific Error Types.
//!
//! Defines the `PriceError` enum used throughout the `rutengri` crate for error propagation.
//!
//! # Implementation Details
//!
//! *   **`thiserror`:** Uses the `thiserror` crate to derive `Error` traits and formatted display strings.
//! *   **Scope:** Covers database issues, price unavailability, invalid inputs, and IO errors.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PriceError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Price not available for pair: {0}")]
    PriceNotAvailable(String),

    #[error("Invalid block number: {0}")]
    InvalidBlockNumber(u64),

    #[error("Source not supported: {0}")]
    SourceNotSupported(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Reader not initialized: {0}")]
    ReaderNotInitialized(String),

    #[error("Unsupported trading pair: {0}")]
    UnsupportedPair(String),
}
