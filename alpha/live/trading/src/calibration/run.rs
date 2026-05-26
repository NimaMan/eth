use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::json;

use super::{
    CalibrationCaseInput, CalibrationCaseReport, CalibrationInputFile, CalibrationReport,
    CalibrationRunConfig, CalibrationSubmitReport, CalibrationSuiteInput, CalibrationSummary,
    ExpectedCalibrationOutcome,
};
use crate::kartal::{
    KartalClient, KartalClientConfig, KartalClientError, KartalEthTxExecutorStatus,
    KartalPolicyDecision, KartalServerError,
};

pub async fn run_calibration(
    config: CalibrationRunConfig,
    input: CalibrationInputFile,
    default_expect: ExpectedCalibrationOutcome,
) -> Result<CalibrationReport, KartalClientError> {
    let suite = input.into_suite(default_expect);
    let client = KartalClient::new(KartalClientConfig::new(
        config.kartal_base_url.clone(),
        config.bearer_token.clone(),
    ));
    run_calibration_with_client(config, suite, client).await
}

async fn run_calibration_with_client(
    config: CalibrationRunConfig,
    suite: CalibrationSuiteInput,
    client: KartalClient,
) -> Result<CalibrationReport, KartalClientError> {
    let started_at_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let status = client.eth_tx_status().await?;
    let preflight_issues = preflight_issues(&status, config.require_dry_run);
    let latest_block_number = if config.refresh_simulation_block && preflight_issues.is_empty() {
        Some(fetch_latest_block_number(&status.rpc_url).await?)
    } else {
        None
    };

    let mut cases = Vec::new();
    for mut case in suite.cases {
        ensure_attempt_id(&mut case);
        if let Some(block_number) = latest_block_number {
            refresh_case_simulation_block(&mut case, block_number);
        }
        if !preflight_issues.is_empty() {
            cases.push(CalibrationCaseReport::skipped(
                &case,
                preflight_issues.join("; "),
            ));
            continue;
        }
        cases.push(run_case(&client, &config, case).await);
    }

    let summary = CalibrationSummary::from_cases(&cases);
    Ok(CalibrationReport {
        suite_name: suite.name,
        started_at_unix_seconds,
        kartal_base_url: config.kartal_base_url,
        status: Some(status),
        preflight_issues,
        cases,
        summary,
    })
}

async fn run_case(
    client: &KartalClient,
    config: &CalibrationRunConfig,
    case: CalibrationCaseInput,
) -> CalibrationCaseReport {
    let attempt_id = case
        .request
        .attempt_id
        .clone()
        .unwrap_or_else(|| "missing-attempt-id".to_string());

    let submit = match client.submit_direct_raw(&case.request).await {
        Ok(result) => CalibrationSubmitReport::Success { result },
        Err(KartalClientError::Server(error)) => CalibrationSubmitReport::from(error),
        Err(error) => CalibrationSubmitReport::ClientError {
            error: error.to_string(),
        },
    };

    let policy_decisions = if config.fetch_policy_journal {
        match client.policy_decisions_for_attempt(&attempt_id).await {
            Ok(list) => list.decisions,
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let verdict = verdict_for_case(&case.expect, &submit, policy_decisions.first());

    CalibrationCaseReport {
        name: case.name,
        attempt_id,
        expect: case.expect,
        submit,
        policy_decisions,
        verdict,
    }
}

fn preflight_issues(status: &KartalEthTxExecutorStatus, require_dry_run: bool) -> Vec<String> {
    let mut issues = Vec::new();
    if require_dry_run && !status.broadcast_mode.is_dry_run() {
        issues.push(format!(
            "Kartal broadcast_mode is {:?}; calibration requires dry_run",
            status.broadcast_mode
        ));
    }
    if status.execution_disabled {
        issues.push("Kartal execution kill switch is active".to_string());
    }
    if !status.api_token_configured {
        issues.push("Kartal reports no API token configured".to_string());
    }
    issues
}

fn ensure_attempt_id(case: &mut CalibrationCaseInput) {
    let missing = case
        .request
        .attempt_id
        .as_deref()
        .map(str::trim)
        .filter(|attempt_id| !attempt_id.is_empty())
        .is_none();
    if missing {
        case.request.attempt_id = Some(format!(
            "kartal-calibration-{}",
            stable_request_suffix(&case.name, &case.request.to, &case.request.data)
        ));
    }
}

fn refresh_case_simulation_block(case: &mut CalibrationCaseInput, latest_block_number: u64) {
    if let Some(simulation) = case.request.simulation.as_mut() {
        simulation.block_number = latest_block_number;
    }
}

async fn fetch_latest_block_number(rpc_url: &str) -> Result<u64, KartalClientError> {
    let response = reqwest::Client::new()
        .post(rpc_url)
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_blockNumber",
            "params": [],
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<RpcBlockNumberResponse>()
        .await?;

    let block_hex = response.result.trim();
    let block_hex = block_hex.strip_prefix("0x").unwrap_or(block_hex);
    u64::from_str_radix(block_hex, 16).map_err(|error| {
        KartalClientError::Config(format!(
            "failed to decode eth_blockNumber response {:?}: {error}",
            response.result
        ))
    })
}

#[derive(Debug, Deserialize)]
struct RpcBlockNumberResponse {
    result: String,
}

fn stable_request_suffix(name: &str, to: &str, data: &str) -> u64 {
    let mut hash = 14_695_981_039_346_656_037u64;
    for byte in name.bytes().chain(to.bytes()).chain(data.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

fn verdict_for_case(
    expect: &ExpectedCalibrationOutcome,
    submit: &CalibrationSubmitReport,
    latest_decision: Option<&KartalPolicyDecision>,
) -> super::CalibrationCaseVerdict {
    match expect {
        ExpectedCalibrationOutcome::PolicyRejected => {
            let Some(decision) = latest_decision else {
                return super::CalibrationCaseVerdict::failed(
                    "expected policy rejection but found no policy journal decision",
                );
            };
            if decision.is_rejected_or_disabled() {
                super::CalibrationCaseVerdict::passed(format!(
                    "Kartal rejected before signing with policy decision {}",
                    decision.decision
                ))
            } else {
                super::CalibrationCaseVerdict::failed(format!(
                    "expected rejected/disabled policy decision, got {}",
                    decision.decision
                ))
            }
        }
        ExpectedCalibrationOutcome::DryRunSigned => match submit {
            CalibrationSubmitReport::Success { result } if result.status == "dry_run" => {
                match latest_decision {
                    Some(decision) if decision.is_accepted() => {
                        super::CalibrationCaseVerdict::passed(
                            "Kartal accepted policy and tx_executor returned dry_run",
                        )
                    }
                    Some(decision) => super::CalibrationCaseVerdict::failed(format!(
                        "tx_executor returned dry_run but latest policy decision was {}",
                        decision.decision
                    )),
                    None => super::CalibrationCaseVerdict::failed(
                        "tx_executor returned dry_run but no policy decision was found",
                    ),
                }
            }
            CalibrationSubmitReport::Success { result } => super::CalibrationCaseVerdict::failed(
                format!("expected dry_run status, got {}", result.status),
            ),
            CalibrationSubmitReport::ServerError { status, body } => {
                super::CalibrationCaseVerdict::failed(format!(
                    "expected dry_run signing but Kartal returned HTTP {status}: {body}"
                ))
            }
            CalibrationSubmitReport::ClientError { error } => {
                super::CalibrationCaseVerdict::failed(format!(
                    "expected dry_run signing but client failed: {error}"
                ))
            }
            CalibrationSubmitReport::Skipped { reason } => {
                super::CalibrationCaseVerdict::unsafe_to_submit(format!(
                    "case skipped before dry-run signing: {reason}"
                ))
            }
        },
        ExpectedCalibrationOutcome::AnyDecision => {
            if latest_decision.is_some() {
                super::CalibrationCaseVerdict::passed("Kartal wrote a policy decision")
            } else {
                super::CalibrationCaseVerdict::failed("no policy decision was found")
            }
        }
    }
}

#[allow(dead_code)]
fn _server_error_report(error: KartalServerError) -> CalibrationSubmitReport {
    CalibrationSubmitReport::from(error)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::kartal::{
        KartalDailySpendStatus, KartalEthTxPolicyStatus, KartalStatusBroadcastMode,
    };
    use crate::CalibrationVerdictKind;

    fn status(mode: KartalStatusBroadcastMode) -> KartalEthTxExecutorStatus {
        KartalEthTxExecutorStatus {
            service: "eth_tx_executor".to_string(),
            enabled: false,
            api_token_configured: true,
            signer_available: false,
            execution_disabled: false,
            broadcast_mode: mode,
            chain_id: 1,
            rpc_url: "http://127.0.0.1:8545".to_string(),
            journal_path: None,
            direct_raw_endpoint: "/eth/tx/direct-raw".to_string(),
            submit_endpoint: Some("/eth/tx/submit".to_string()),
            flashbots_tail_bundle_endpoint: Some("/eth/tx/flashbots/mev-share-tail".to_string()),
            flashbots_relay_url: Some("https://relay.flashbots.net".to_string()),
            flashbots_auth_configured: Some(false),
            policy: KartalEthTxPolicyStatus {
                version: "test".to_string(),
                allowed_from_count: 0,
                allowed_target_count: 0,
                allowed_selector_count: 0,
                max_value_wei: "0".to_string(),
                max_gas_limit: 500000,
                max_fee_per_gas_wei: "1".to_string(),
                max_priority_fee_per_gas_wei: "1".to_string(),
                max_transaction_cost_wei: "0".to_string(),
                max_daily_cost_wei: "0".to_string(),
                daily_spend_cap_enabled: Some(false),
                daily_spend: KartalDailySpendStatus {
                    spend_day: "2026-05-19".to_string(),
                    spent_wei: "0".to_string(),
                    remaining_daily_cost_wei: None,
                },
                require_simulation: true,
                max_simulation_age_blocks: 2,
                required_metadata_fields: Vec::new(),
            },
        }
    }

    fn decision(decision: &str) -> KartalPolicyDecision {
        serde_json::from_value(json!({
            "id": 1,
            "attempt_id": "attempt-1",
            "created_at": "2026-05-19T00:00:00Z",
            "route": "/eth/tx/direct-raw",
            "decision": decision,
            "policy_version": "test",
            "reasons": [],
            "request": {},
            "normalized": {},
            "metadata": {},
            "strategy_name": null,
            "strategy_run_id": null,
            "trade_id": null,
            "token_address": null,
            "pool_address": null,
            "from_address": null,
            "to_address": null,
            "selector": null,
            "value_wei": null,
            "gas_limit": null,
            "max_fee_per_gas_wei": null,
            "max_priority_fee_per_gas_wei": null,
            "estimated_worst_case_cost_wei": null,
            "simulation_block_number": null,
            "latest_block_number": null
        }))
        .unwrap()
    }

    #[test]
    fn preflight_rejects_non_dry_run_by_default() {
        let issues = preflight_issues(&status(KartalStatusBroadcastMode::PublicMempool), true);
        assert!(issues.iter().any(|issue| issue.contains("dry_run")));
    }

    #[test]
    fn refreshes_existing_simulation_block() {
        let mut case: CalibrationCaseInput = serde_json::from_value(json!({
            "name": "case",
            "request": {
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
                "simulation": {
                    "block_number": 1,
                    "block_hash": null,
                    "state_root": null,
                    "expected_output_token": null,
                    "expected_output_amount": null,
                    "min_output_amount": null,
                    "metadata": {}
                },
                "metadata": {}
            }
        }))
        .unwrap();

        refresh_case_simulation_block(&mut case, 123);

        assert_eq!(case.request.simulation.unwrap().block_number, 123);
    }

    #[test]
    fn policy_rejected_expectation_passes_on_rejected_decision() {
        let verdict = verdict_for_case(
            &ExpectedCalibrationOutcome::PolicyRejected,
            &CalibrationSubmitReport::ServerError {
                status: 400,
                body: "rejected".to_string(),
            },
            Some(&decision("rejected")),
        );
        assert_eq!(verdict.kind, CalibrationVerdictKind::Passed);
    }
}
