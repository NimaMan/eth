pub mod checks;
pub mod db;
pub mod persistence;
pub mod report;
pub mod runner;

pub use report::{BacktestValidationReport, CheckResult, TradeSample, ValidationSummary, Verdict};
pub use runner::{validate_backtest, ValidationOptions, COMPREHENSIVE_VALIDATION_PROFILE};
