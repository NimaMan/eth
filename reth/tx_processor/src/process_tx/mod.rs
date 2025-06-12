//! Transaction processing module
//! 
//! This module handles the processing of simulated transactions,
//! extracting state changes and calculating final account states.
//! 
//! ## Python Integration
//! 
//! The module provides Python-compatible state change extraction that can be
//! directly compared with the Python validation service running on port 18000.

pub mod state_diff_utils;
pub mod python_validator;

#[cfg(test)]
pub mod process_tx_tests;

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
    // Python-compatible exports
    ProcessTxError,
    PythonCompatibleStateChanges,
    AddressStateChange,
    ProcessingMetadata,
    EventCounts,
    extract_state_changes_python_format,
    extract_batch_state_changes_python_format,
    convert_to_python_format,
    format_token_amount,
    format_eth_amount,
    extract_event_counts,
};

// Re-export python validator types
pub use python_validator::{
    PythonValidatorClient,
    ValidationRequest,
    BatchValidationRequest,
    PythonValidationResponse,
    PythonBatchValidationResponse,
    TransactionSummary,
    HealthStatus,
    PythonProcessedTransaction,
    ValidationResult,
    ValidationDifference,
    compare_with_python,
    batch_compare_with_python,
};