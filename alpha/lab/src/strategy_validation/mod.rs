pub mod checks;
pub mod db;
pub mod persistence;
pub mod report;
pub mod runner;

pub use report::{CheckResult, StrategyValidationReport, TradeSample, ValidationSummary, Verdict};
pub use runner::{validate_strategy, ValidationOptions, COMPREHENSIVE_VALIDATION_PROFILE};
