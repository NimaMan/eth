//! Shared Transaction Simulator.
//!
//! Provides a global singleton instance of the `TxSimulator` to manage database access.
//!
//! # Implementation Details
//!
//! *   **Singleton Pattern:** Uses `lazy_static` and `Mutex` to ensure only one `TxSimulator` instance exists.
//! *   **MDBX Safety:** Prevents "Error 11" (Resource temporarily unavailable) which occurs when multiple processes/threads try to open the same MDBX environment with write flags (or even read flags depending on configuration).
//! *   **Provider Factory:** Reuses the existing `ProviderFactory` to avoid opening new DB handles.

use crate::core::PriceError;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
/// Shared simulator singleton for all readers that need view function calls
///
/// This module provides a single shared TxSimulator instance to avoid
/// the "error code 11" issue that occurs when multiple simulators try to
/// access the same database.
use tx_simulator::TxSimulator;

// Global singleton simulator - shared by ALL readers
lazy_static! {
    static ref SHARED_SIMULATOR: Mutex<Option<Arc<TxSimulator>>> = Mutex::new(None);
}

/// Get or create the shared simulator instance using an existing provider factory
/// This ensures we use the SAME database connection as all other components
pub fn get_or_create_simulator_with_provider(
    provider_factory: Arc<crate::utils::EthPriceProviderFactory>,
) -> Result<Arc<TxSimulator>, PriceError> {
    let mut guard = SHARED_SIMULATOR.lock().unwrap();

    if let Some(ref simulator) = *guard {
        Ok(simulator.clone())
    } else {
        // Use the shared provider factory - NO new database connection!
        // Need to extract the inner ProviderFactory from the Arc
        let provider_factory_clone =
            Arc::try_unwrap(provider_factory.clone()).unwrap_or_else(|arc| (*arc).clone());
        let simulator = Arc::new(
            TxSimulator::with_provider_factory(provider_factory_clone).map_err(|e| {
                PriceError::DatabaseError(format!("Failed to create simulator: {}", e))
            })?,
        );
        *guard = Some(simulator.clone());
        Ok(simulator)
    }
}

/// Compatibility helper that creates its own database connection.
/// DEPRECATED: Use get_or_create_simulator_with_provider instead
pub fn get_or_create_simulator(db_path: &str) -> Result<Arc<TxSimulator>, PriceError> {
    let mut guard = SHARED_SIMULATOR.lock().unwrap();

    if let Some(ref simulator) = *guard {
        Ok(simulator.clone())
    } else {
        let simulator = Arc::new(TxSimulator::new(db_path).map_err(|e| {
            PriceError::DatabaseError(format!("Failed to create simulator: {}", e))
        })?);
        *guard = Some(simulator.clone());
        Ok(simulator)
    }
}
