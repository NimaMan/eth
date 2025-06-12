pub mod conversions;
pub mod simulate_signed_tx;
pub mod process_tx;
// pub mod fast_path_processor; // Disabled due to missing file
pub mod fetch_from_reth;
// pub mod tx_processor; // Disabled - needs API updates for external REVM but core simulation works

// Re-export common types for easier use in examples or by other crates
pub use conversions::*;

// Re-export simulate_signed_tx module types
pub use simulate_signed_tx::{
    simulate_transaction, simulate_transaction_with_config,
    SimulationConfig, SimulationOutput, SimulationError,
    BlockEnv, CallTracer, CallTrace, CallType,
    InternalTransfer, InternalTransferTracker,
};

// For backward compatibility with old imports
pub use simulate_signed_tx::simulation_core::{SimCacheDB, ExecutionResultType};

// Re-export state diff utilities from process_tx module
pub use process_tx::{
    // Python-compatible state change extraction
    ProcessTxError,
    PythonCompatibleStateChanges,
    AddressStateChange,
    ProcessingMetadata,
    EventCounts,
    extract_state_changes_python_format,
    convert_to_python_format,
    format_token_amount,
    format_eth_amount,
    // Python validator integration
    PythonValidatorClient,
    ValidationResult,
    ValidationDifference,
    compare_with_python,
    batch_compare_with_python,
};

// Re-export fast path processor
// pub use fast_path_processor::{
//     FastPathConfig, FastPathProcessor, TransactionAnalysisResult, 
//     AnalysisMethod, ConfidenceLevel, example_fast_analysis
// };

// Re-export spec utils from simulate_signed_tx
pub use simulate_signed_tx::spec_utils::spec_id_from_block_number;

// Re-export internal transfer integration functions
// Temporarily disabled due to dependency on process_tx module
/*
pub use simulate_signed_tx::internal_transfer_tracker::{
    integrate_internal_transfers
};
*/

// Re-export fetch_from_reth module for direct Reth database access
pub use fetch_from_reth::{
    RethDataProvider, RethDatabaseProvider,
    TransactionData, RethDataConfig, CacheConfig, TransactionCache,
    FetchError, FetchResult, CacheStats
};

// Re-export SharedRethDataProvider from provider submodule
pub use fetch_from_reth::provider::SharedRethDataProvider;

// Conversion functions have been moved to src/conversions.rs

#[cfg(test)]
mod tests {
    // Tests for conversion functions are now in src/conversions.rs
    // If there were other lib-specific tests, they would remain here.
    // For example, a test for hello_from_lib if it existed.
} 