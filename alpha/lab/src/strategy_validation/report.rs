use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::render;

use super::db::{ResultSetRecord, StrategySummary};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyValidationReport {
    pub result_set: ResultSetRecord,
    pub strategy_filter: Option<String>,
    pub profile: String,
    pub summary: ValidationSummary,
    pub strategy_summaries: Vec<StrategySummary>,
    pub checks: Vec<CheckResult>,
    pub samples: Vec<TradeSample>,
    pub procedure: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub checks: usize,
    pub passed: usize,
    pub warnings: usize,
    pub failures: usize,
    pub blocked: usize,
    /// Count of checks that BOTH carry a promotion-blocking classification
    /// (`CheckResult.blocking == true`) AND returned `Verdict::Fail`. A
    /// `Verdict::Blocked` ("could not evaluate") check never contributes here,
    /// so a vacuous historical `tail_entry_coverage=Blocked` does not gate.
    /// `blocking_failures > 0` is the single hard promotion/broadcast blocker.
    #[serde(default)]
    pub blocking_failures: usize,
}

impl ValidationSummary {
    pub fn from_checks(checks: &[CheckResult]) -> Self {
        let mut summary = Self {
            checks: checks.len(),
            ..Self::default()
        };
        for check in checks {
            match check.verdict {
                Verdict::Pass => summary.passed += 1,
                Verdict::Warn => summary.warnings += 1,
                Verdict::Fail => {
                    summary.failures += 1;
                    if check.blocking {
                        summary.blocking_failures += 1;
                    }
                }
                Verdict::Blocked => summary.blocked += 1,
            }
        }
        summary
    }

    /// True when at least one promotion-blocking check hard-failed. This is the
    /// gate predicate consumed by the CLI exit code and the live-real preflight.
    pub fn has_blocking_failures(&self) -> bool {
        self.blocking_failures > 0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckResult {
    pub category: String,
    pub code: String,
    pub question: String,
    pub description: String,
    pub verdict: Verdict,
    pub message: String,
    pub evidence: Value,
    /// When true, a `Verdict::Fail` on this check is a HARD promotion/broadcast
    /// blocker (counts toward `ValidationSummary.blocking_failures`). When
    /// false, the check is advisory/coverage-only and never gates promotion.
    /// Classified centrally in `checks::common::is_blocking_code` keyed on
    /// `code`. Defaults to `true` on deserialization so older persisted reports
    /// (written before this field existed) are treated correctness-first.
    #[serde(default = "default_blocking")]
    pub blocking: bool,
}

fn default_blocking() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Warn,
    Fail,
    Blocked,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeSample {
    pub sample_kind: String,
    pub trade_id: String,
    pub strategy_name: String,
    pub state: String,
    pub token_address: String,
    pub pool_address: String,
    pub entry_block: Option<i64>,
    pub exit_block: Option<i64>,
    pub entry_cost_eth: Option<String>,
    pub exit_value_eth: Option<String>,
    pub realized_pnl_eth: Option<String>,
    pub unrealized_pnl_eth: Option<String>,
    pub total_pnl_eth: Option<String>,
    pub roi: Option<String>,
}

pub fn print_strategy_validation_report(report: &StrategyValidationReport) {
    println!("# Strategy Validation: {}", report.result_set.result_set_id);
    println!();
    println!(
        "- mode/status: `{}` / `{}`",
        report.result_set.mode, report.result_set.status
    );
    println!(
        "- strategy suite: `{}`",
        report
            .result_set
            .strategy_suite
            .as_deref()
            .unwrap_or("<none>")
    );
    println!(
        "- block range: `{}` - `{}`",
        report
            .result_set
            .start_block
            .map(|block| block.to_string())
            .unwrap_or_else(|| "-".to_string()),
        report
            .result_set
            .end_block
            .map(|block| block.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "- strategy filter: `{}`",
        report.strategy_filter.as_deref().unwrap_or("<all>")
    );
    println!("- validation: `{}`", report.profile);
    println!();

    println!("## Verdict Summary");
    render::print_table(
        &["metric", "count"],
        &[
            vec!["checks".to_string(), report.summary.checks.to_string()],
            vec!["passed".to_string(), report.summary.passed.to_string()],
            vec!["warnings".to_string(), report.summary.warnings.to_string()],
            vec!["failures".to_string(), report.summary.failures.to_string()],
            vec!["blocked".to_string(), report.summary.blocked.to_string()],
            vec![
                "blocking_failures".to_string(),
                report.summary.blocking_failures.to_string(),
            ],
        ],
    );
    if report.summary.has_blocking_failures() {
        println!();
        println!(
            "PROMOTION GATE: BLOCKED - {} promotion-blocking check(s) failed; this strategy/result-set MUST NOT be promoted or broadcast.",
            report.summary.blocking_failures
        );
    } else {
        println!();
        println!("PROMOTION GATE: clear - no promotion-blocking check failed.");
    }
    println!();

    println!("## Strategies");
    if report.strategy_summaries.is_empty() {
        println!("No strategies matched this validation scope.");
    } else {
        render::print_table(
            &[
                "strategy",
                "trades",
                "closed",
                "open",
                "failed",
                "realized",
                "unrealized",
                "total",
            ],
            &report
                .strategy_summaries
                .iter()
                .map(|summary| {
                    vec![
                        summary.strategy_name.clone(),
                        summary.trades.to_string(),
                        summary.closed.to_string(),
                        summary.open.to_string(),
                        summary.failed.to_string(),
                        summary.realized_pnl_eth.clone(),
                        summary.unrealized_pnl_eth.clone(),
                        summary.total_pnl_eth.clone(),
                    ]
                })
                .collect::<Vec<_>>(),
        );
    }
    println!();

    println!("## Checks");
    render::print_table(
        &["verdict", "gate", "category", "question", "message"],
        &report
            .checks
            .iter()
            .map(|check| {
                let gate = if check.blocking { "blocking" } else { "advisory" };
                vec![
                    check.verdict.as_str().to_string(),
                    gate.to_string(),
                    check.category.clone(),
                    check.question.clone(),
                    check.message.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    );
    println!();

    println!("## Trade Samples");
    if report.samples.is_empty() {
        println!("No trade samples matched this validation scope.");
    } else {
        render::print_table(
            &[
                "sample",
                "trade",
                "strategy",
                "state",
                "entry",
                "exit",
                "total PnL",
                "ROI",
            ],
            &report
                .samples
                .iter()
                .map(|sample| {
                    vec![
                        sample.sample_kind.clone(),
                        short(&sample.trade_id),
                        short(&sample.strategy_name),
                        sample.state.clone(),
                        sample
                            .entry_block
                            .map(|block| block.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        sample
                            .exit_block
                            .map(|block| block.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        sample
                            .total_pnl_eth
                            .clone()
                            .unwrap_or_else(|| "-".to_string()),
                        sample.roi.clone().unwrap_or_else(|| "-".to_string()),
                    ]
                })
                .collect::<Vec<_>>(),
        );
    }
}

fn short(value: &str) -> String {
    if value.len() <= 22 {
        return value.to_string();
    }
    format!("{}...{}", &value[..12], &value[value.len() - 6..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn check(verdict: Verdict, blocking: bool) -> CheckResult {
        CheckResult {
            category: "test".to_string(),
            code: "synthetic".to_string(),
            question: String::new(),
            description: String::new(),
            verdict,
            message: String::new(),
            evidence: json!({}),
            blocking,
        }
    }

    #[test]
    fn blocking_fail_counts_as_blocking_failure_and_gates() {
        let summary = ValidationSummary::from_checks(&[
            check(Verdict::Pass, true),
            check(Verdict::Fail, true),
        ]);
        assert_eq!(summary.failures, 1);
        assert_eq!(summary.blocking_failures, 1);
        assert!(summary.has_blocking_failures());
    }

    #[test]
    fn advisory_fail_does_not_gate() {
        let summary = ValidationSummary::from_checks(&[check(Verdict::Fail, false)]);
        assert_eq!(summary.failures, 1);
        assert_eq!(summary.blocking_failures, 0);
        assert!(!summary.has_blocking_failures());
    }

    #[test]
    fn blocked_verdict_never_gates_even_when_classified_blocking() {
        // A historical `tail_entry_coverage` returns Blocked; even if some
        // future blocking-classified check returns Blocked it must not gate,
        // because Blocked means "could not evaluate", not "failed".
        let summary = ValidationSummary::from_checks(&[
            check(Verdict::Blocked, true),
            check(Verdict::Blocked, false),
        ]);
        assert_eq!(summary.blocked, 2);
        assert_eq!(summary.blocking_failures, 0);
        assert!(!summary.has_blocking_failures());
    }

    #[test]
    fn warnings_and_passes_never_gate() {
        let summary = ValidationSummary::from_checks(&[
            check(Verdict::Warn, true),
            check(Verdict::Pass, true),
        ]);
        assert_eq!(summary.warnings, 1);
        assert_eq!(summary.passed, 1);
        assert!(!summary.has_blocking_failures());
    }

    #[test]
    fn blocking_field_defaults_true_for_legacy_reports() {
        // Reports persisted before `blocking` existed deserialize with the
        // field absent; serde must default it to true (correctness-first).
        let legacy = json!({
            "category": "accounting",
            "code": "total_pnl_equals_realized_plus_unrealized",
            "question": "",
            "description": "",
            "verdict": "fail",
            "message": "",
            "evidence": {}
        });
        let parsed: CheckResult = serde_json::from_value(legacy).unwrap();
        assert!(parsed.blocking);
    }
}
