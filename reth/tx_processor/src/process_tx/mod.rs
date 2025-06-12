//! Transaction processing module
//! 
//! This module handles the processing of simulated transactions,
//! extracting state changes and calculating final account states.

pub mod state_diff_utils;

// Re-export commonly used types
pub use state_diff_utils::{
    AccountStatusInDiff,
    AccountStateSummary,
    extract_final_touched_account_states,
    SimCacheDBForDiff,
    CalculatedAccountChanges,
    generate_calculated_account_changes,
    TokenInfo,
    get_token_symbol,
    get_token_decimals,
};