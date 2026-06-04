use serde::{Deserialize, Serialize};

use super::ExpectedCalibrationOutcome;
use crate::LiveDirectRawTransactionRequest;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CalibrationInputFile {
    Suite(CalibrationSuiteInput),
    Single(LiveDirectRawTransactionRequest),
}

impl CalibrationInputFile {
    pub fn into_suite(self, default_expect: ExpectedCalibrationOutcome) -> CalibrationSuiteInput {
        match self {
            Self::Suite(suite) => suite,
            Self::Single(request) => CalibrationSuiteInput {
                name: "single-request".to_string(),
                cases: vec![CalibrationCaseInput {
                    name: request
                        .attempt_id
                        .clone()
                        .unwrap_or_else(|| "single-request".to_string()),
                    expect: default_expect,
                    request,
                }],
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationSuiteInput {
    #[serde(default = "default_suite_name")]
    pub name: String,
    pub cases: Vec<CalibrationCaseInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalibrationCaseInput {
    #[serde(default = "default_case_name")]
    pub name: String,
    #[serde(default)]
    pub expect: ExpectedCalibrationOutcome,
    pub request: LiveDirectRawTransactionRequest,
}

fn default_suite_name() -> String {
    "eth-tx-calibration".to_string()
}

fn default_case_name() -> String {
    "case".to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn single_request_file_becomes_one_case_suite() {
        let input: CalibrationInputFile = serde_json::from_value(json!({
            "attempt_id": "attempt-1",
            "chain_id": 1,
            "from": "0x0000000000000000000000000000000000000001",
            "to": "0x0000000000000000000000000000000000000002",
            "value": "0",
            "data": "0x095ea7b3",
            "gas_limit": "21000",
            "max_fee_per_gas": "1000000000",
            "max_priority_fee_per_gas": "1000000000",
            "nonce": null,
            "bribe": null,
            "simulation": null,
            "metadata": {}
        }))
        .unwrap();

        let suite = input.into_suite(ExpectedCalibrationOutcome::PolicyRejected);
        assert_eq!(suite.cases.len(), 1);
        assert_eq!(suite.cases[0].name, "attempt-1");
    }
}
