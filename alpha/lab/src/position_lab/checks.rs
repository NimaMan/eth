use crate::{is_nonzero_hex, parse_f64, RunMetadata};

use super::model::{
    ExecutionReportRecord, PoolObservation, PositionCheck, PositionRecord, SnapshotRecord,
};

pub(super) fn build_checks(
    run: &RunMetadata,
    position: &PositionRecord,
    entry_report: Option<&ExecutionReportRecord>,
    latest_snapshot: Option<&SnapshotRecord>,
    entry_observation: Option<&PoolObservation>,
    latest_observation: Option<&PoolObservation>,
) -> Vec<PositionCheck> {
    let mut checks = Vec::new();

    push_check(
        &mut checks,
        run.replay_run_id.is_some(),
        "replay_run_recorded",
        "run metadata records replay_run_id for observation joins",
        "run metadata has no replay_run_id; pass --replay-run-id for observation joins",
    );

    match entry_report {
        Some(report) => {
            push_check(
                &mut checks,
                report.status == "confirmed",
                "entry_confirmed",
                "entry execution report is confirmed",
                "entry execution report is not confirmed",
            );
            if let (Some(report_block), Some(entry_block)) =
                (report.block_number, position.entry_block)
            {
                push_check(
                    &mut checks,
                    report_block == entry_block,
                    "entry_block_match",
                    "entry report block matches position entry_block",
                    "entry report block does not match position entry_block",
                );
            }
            push_check(
                &mut checks,
                report.filled_amount_raw.is_some(),
                "entry_cost_recorded",
                "entry report has filled ETH amount",
                "entry report is missing filled ETH amount",
            );
            push_check(
                &mut checks,
                report
                    .token_amount_raw
                    .as_deref()
                    .map(|raw| is_nonzero_hex(Some(raw)))
                    .unwrap_or(false),
                "entry_token_raw_nonzero",
                "entry report has nonzero raw token amount",
                "entry report raw token amount is missing or zero",
            );
        }
        None => checks.push(PositionCheck {
            status: "fail".to_string(),
            code: "entry_report_missing".to_string(),
            message: "position entry_order_id has no execution report".to_string(),
        }),
    }

    let decimal_zero = position
        .entry_token_amount
        .as_deref()
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value == 0.0)
        .unwrap_or(false);
    let raw_nonzero = is_nonzero_hex(position.entry_token_raw.as_deref());
    if decimal_zero && raw_nonzero {
        checks.push(PositionCheck {
            status: "warn".to_string(),
            code: "zero_decimal_nonzero_raw".to_string(),
            message:
                "entry_token_amount is zero but entry_token_raw_amount is nonzero; decimal display likely overflowed"
                    .to_string(),
        });
    } else {
        checks.push(PositionCheck {
            status: "pass".to_string(),
            code: "decimal_raw_consistency".to_string(),
            message: "decimal token amount is consistent with raw amount".to_string(),
        });
    }

    push_check(
        &mut checks,
        latest_snapshot.is_some(),
        "has_snapshot",
        "position has at least one mark-to-market snapshot",
        "position has no mark-to-market snapshot",
    );

    match entry_observation {
        Some(obs) => {
            push_check(
                &mut checks,
                obs.can_buy == Some(true) && obs.can_sell == Some(true),
                "entry_tradable",
                "entry observation is buyable and sellable",
                "entry observation is not buyable and sellable",
            );
            let denom_reserve = parse_f64(&obs.denom_reserve);
            push_check(
                &mut checks,
                denom_reserve.map(|reserve| reserve >= 0.5).unwrap_or(false),
                "entry_liquidity_threshold",
                "entry denom reserve is at or above 0.5 ETH/WETH",
                "entry denom reserve is below threshold or unavailable",
            );
        }
        None => checks.push(PositionCheck {
            status: "warn".to_string(),
            code: "entry_observation_missing".to_string(),
            message: "could not find matching entry pool observation".to_string(),
        }),
    }

    if let (Some(snapshot), Some(obs)) = (latest_snapshot, latest_observation) {
        let value = parse_f64(&snapshot.current_value_eth).unwrap_or_default();
        if value == 0.0 && obs.can_sell == Some(false) {
            checks.push(PositionCheck {
                status: "pass".to_string(),
                code: "zero_value_explained_by_untradable_pool".to_string(),
                message: "latest value is zero and latest observation is not sellable".to_string(),
            });
        }
    }

    checks
}

fn push_check(
    checks: &mut Vec<PositionCheck>,
    passed: bool,
    code: &str,
    pass_message: &str,
    fail_message: &str,
) {
    checks.push(PositionCheck {
        status: if passed { "pass" } else { "fail" }.to_string(),
        code: code.to_string(),
        message: if passed { pass_message } else { fail_message }.to_string(),
    });
}
