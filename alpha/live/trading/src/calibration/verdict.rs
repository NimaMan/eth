use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedCalibrationOutcome {
    /// Diagnostic mode: require a policy journal decision, but accept either
    /// policy rejection or dry-run signing.
    AnyDecision,
    /// Safe default for always-runnable checks with a reject-by-default ETH tx executor
    /// policy. No signer is required because ETH tx executor rejects before signing.
    PolicyRejected,
    /// Full dry-run path: ETH tx executor policy accepts the request and tx_executor
    /// returns `dry_run`, proving decode -> policy -> signing -> journaling.
    DryRunSigned,
}

impl Default for ExpectedCalibrationOutcome {
    fn default() -> Self {
        Self::PolicyRejected
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationVerdictKind {
    Passed,
    Failed,
    Unsafe,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CalibrationCaseVerdict {
    pub kind: CalibrationVerdictKind,
    pub reason: String,
}

impl CalibrationCaseVerdict {
    pub fn passed(reason: impl Into<String>) -> Self {
        Self {
            kind: CalibrationVerdictKind::Passed,
            reason: reason.into(),
        }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            kind: CalibrationVerdictKind::Failed,
            reason: reason.into(),
        }
    }

    pub fn unsafe_to_submit(reason: impl Into<String>) -> Self {
        Self {
            kind: CalibrationVerdictKind::Unsafe,
            reason: reason.into(),
        }
    }

    pub fn is_passed(&self) -> bool {
        self.kind == CalibrationVerdictKind::Passed
    }

    pub fn is_unsafe(&self) -> bool {
        self.kind == CalibrationVerdictKind::Unsafe
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationOverallVerdict {
    Passed,
    Failed,
    Unsafe,
}
