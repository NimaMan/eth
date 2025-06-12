pub mod conversions;
pub mod simulate_signed_tx;
pub mod process_tx;
pub mod fast_path_processor;
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
pub use process_tx::state_diff_utils::{
    AccountStatusInDiff,
    // StorageSlotDiff, // This struct is now internal to state_diff_utils or not used for summary
    AccountStateSummary, // Renamed from AccountStateDiff
    extract_final_touched_account_states, // Renamed function
    SimCacheDBForDiff,
    CalculatedAccountChanges,
    generate_calculated_account_changes,
    TokenInfo,
    get_token_symbol,
    get_token_decimals,
    // Add other new structs if they need to be public API e.g. SignedAmount, AccountMovements etc.
    // For now, keeping them internal to state_diff_utils unless direct use is needed by examples.
};

// Re-export fast path processor
pub use fast_path_processor::{
    FastPathConfig, FastPathProcessor, TransactionAnalysisResult, 
    AnalysisMethod, ConfidenceLevel, example_fast_analysis
};

// Re-export spec utils from simulate_signed_tx
pub use simulate_signed_tx::spec_utils::spec_id_from_block_number;

// Re-export internal transfer integration functions
pub use simulate_signed_tx::internal_transfer_tracker::{
    integrate_internal_transfers
};

// Conversion functions have been moved to src/conversions.rs

#[cfg(test)]
mod tests {
    // Tests for conversion functions are now in src/conversions.rs
    // If there were other lib-specific tests, they would remain here.
    // For example, a test for hello_from_lib if it existed.
} 