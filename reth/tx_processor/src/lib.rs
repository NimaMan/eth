pub mod conversions;
pub mod simulation_core;
pub mod state_diff_utils;
pub mod call_tracer;
pub mod internal_transfer_tracker;
// pub mod tx_processor; // Disabled - needs API updates for external REVM but core simulation works

// Re-export common types for easier use in examples or by other crates
pub use conversions::*;
pub use simulation_core::{simulate_transaction, SimulationOutput, SimCacheDB, ExecutionResultType};

// Re-export state diff utilities
pub use state_diff_utils::{
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

// Re-export call tracer and internal transfer tracker
pub use call_tracer::{CallTracer, InternalTransfer};
pub use internal_transfer_tracker::{integrate_internal_transfers, extract_internal_transfers_from_rpc};

// Conversion functions have been moved to src/conversions.rs

#[cfg(test)]
mod tests {
    // Tests for conversion functions are now in src/conversions.rs
    // If there were other lib-specific tests, they would remain here.
    // For example, a test for hello_from_lib if it existed.
} 