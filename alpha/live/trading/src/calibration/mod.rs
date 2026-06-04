//! Repeatable ETH tx executor policy-calibration runner.
//!
//! Calibration submits already prepared `eth_unsigned_tx` requests to ETH tx executor
//! while requiring dry-run mode by default. It records the executor status,
//! submit result, policy journal entries, and a verdict for each case.

mod config;
mod input;
mod planner_fixture;
mod report;
mod run;
mod verdict;

pub use config::CalibrationRunConfig;
pub use input::{CalibrationCaseInput, CalibrationInputFile, CalibrationSuiteInput};
pub use planner_fixture::{
    build_planner_calibration_request, PlannerCalibrationFixtureConfig,
    PlannerCalibrationFixtureError, PlannerCalibrationRoute,
};
pub use report::{
    CalibrationCaseReport, CalibrationReport, CalibrationSubmitReport, CalibrationSummary,
};
pub use run::run_calibration;
pub use verdict::{
    CalibrationCaseVerdict, CalibrationOverallVerdict, CalibrationVerdictKind,
    ExpectedCalibrationOutcome,
};
