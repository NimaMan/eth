//! Base Chain Reader Infrastructure.
//!
//! Provides shared functionality for establishing connections to the Reth database.
//!
//! # Implementation Details
//!
//! *   **Provider Factory:** Manages the creation and sharing of `ProviderFactory` to reuse database handles.
//! *   **Read-Only Access:** Opens the MDBX database in read-only mode to prevent corruption and allow concurrent access.
//! *   **Chain Spec:** Initializes with Mainnet chain specification by default.

use crate::core::PriceError;
use std::sync::Arc;

/// Base reader with shared provider factory functionality
pub struct BaseReader {
    pub provider_factory: crate::utils::EthPriceProviderFactory,
}

impl BaseReader {
    /// Create a new BaseReader from a shared provider factory (preferred method)
    pub fn from_provider(
        provider_factory: Arc<crate::utils::EthPriceProviderFactory>,
    ) -> Result<Self, PriceError> {
        Ok(Self {
            provider_factory: (*provider_factory).clone(),
        })
    }

    /// Create a new BaseReader by opening a database connection (only use if no shared provider available)
    pub fn new(db_path: &str) -> Result<Self, PriceError> {
        Self::from_provider(crate::utils::open_provider_factory(db_path)?)
    }
}
