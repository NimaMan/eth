//! Process TX Tests
//! 
//! Comprehensive test suite for the process_tx module, covering:
//! - State change extraction
//! - Python service integration
//! - Real transaction validation
//! - Performance benchmarks

pub mod state_extraction_tests;
pub mod python_integration_tests;
pub mod real_transaction_tests;
pub mod performance_tests;
pub mod comparison_tests;

// Re-export test utilities
pub use state_extraction_tests::*;
pub use python_integration_tests::*;
pub use real_transaction_tests::*;
pub use performance_tests::*;
pub use comparison_tests::*;