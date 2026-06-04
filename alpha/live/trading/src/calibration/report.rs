use serde::{Deserialize, Serialize};

use super::{
    CalibrationCaseInput, CalibrationCaseVerdict, CalibrationOverallVerdict,
    ExpectedCalibrationOutcome,
};
use crate::eth_tx_executor::{
    EthTxExecutorServerError, EthTxExecutorStatus, EthTxPolicyDecision, EthTxSubmitDirectRawResult,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationReport {
    pub suite_name: String,
    pub started_at_unix_seconds: u64,
    pub eth_tx_executor_base_url: String,
    pub status: Option<EthTxExecutorStatus>,
    pub preflight_issues: Vec<String>,
    pub cases: Vec<CalibrationCaseReport>,
    pub summary: CalibrationSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationSummary {
    pub verdict: CalibrationOverallVerdict,
    pub passed: usize,
    pub failed: usize,
    pub unsafe_cases: usize,
    pub total: usize,
}

impl CalibrationSummary {
    pub fn from_cases(cases: &[CalibrationCaseReport]) -> Self {
        let passed = cases.iter().filter(|case| case.verdict.is_passed()).count();
        let unsafe_cases = cases.iter().filter(|case| case.verdict.is_unsafe()).count();
        let failed = cases.len().saturating_sub(passed + unsafe_cases);
        let verdict = if unsafe_cases > 0 {
            CalibrationOverallVerdict::Unsafe
        } else if failed > 0 {
            CalibrationOverallVerdict::Failed
        } else {
            CalibrationOverallVerdict::Passed
        };

        Self {
            verdict,
            passed,
            failed,
            unsafe_cases,
            total: cases.len(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationCaseReport {
    pub name: String,
    pub attempt_id: String,
    pub expect: ExpectedCalibrationOutcome,
    pub submit: CalibrationSubmitReport,
    pub policy_decisions: Vec<EthTxPolicyDecision>,
    pub verdict: CalibrationCaseVerdict,
}

impl CalibrationCaseReport {
    pub fn skipped(case: &CalibrationCaseInput, reason: impl Into<String>) -> Self {
        Self {
            name: case.name.clone(),
            attempt_id: case
                .request
                .attempt_id
                .clone()
                .unwrap_or_else(|| "missing-attempt-id".to_string()),
            expect: case.expect.clone(),
            submit: CalibrationSubmitReport::Skipped {
                reason: reason.into(),
            },
            policy_decisions: Vec::new(),
            verdict: CalibrationCaseVerdict::unsafe_to_submit("preflight failed before submission"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CalibrationSubmitReport {
    Success { result: EthTxSubmitDirectRawResult },
    ServerError { status: u16, body: String },
    ClientError { error: String },
    Skipped { reason: String },
}

impl From<EthTxExecutorServerError> for CalibrationSubmitReport {
    fn from(error: EthTxExecutorServerError) -> Self {
        Self::ServerError {
            status: error.status,
            body: error.body,
        }
    }
}
