/*
 * Validation Testing Infrastructure
 * 
 * ALGORITHMIC DESCRIPTION:
 * This module provides comprehensive testing infrastructure to validate the Rust
 * state change calculator against the Python implementation across multiple transactions.
 * 
 * Key Components:
 * 1. Transaction Fetcher: Gets recent transactions from latest blocks
 * 2. Python Bridge: Executes Python state change calculation and parses results
 * 3. Rust Calculator: Runs our Rust implementation on the same transactions
 * 4. Comparison Engine: Validates results with tolerance for floating-point differences
 * 5. Batch Testing: Processes multiple transactions and generates summary reports
 * 6. Error Analysis: Identifies patterns in discrepancies and suggests fixes
 * 
 * This serves the main objective by ensuring our Rust implementation produces
 * identical results to the proven Python implementation across diverse transaction types.
 */

// TODO: Fix batch_validator to use DebugTraceCallStateDiffCalculator
// pub mod batch_validator;
pub mod python_bridge;
pub mod transaction_fetcher;
pub mod comparison_engine;
// pub mod test_runner;

// pub use batch_validator::BatchValidator;
pub use python_bridge::PythonBridge;
pub use transaction_fetcher::TransactionFetcher;
pub use comparison_engine::ComparisonEngine;
// pub use test_runner::TestRunner; 