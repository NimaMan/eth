use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::render;

use super::db::{ResultSetRecord, StrategySummary};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestValidationReport {
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
                Verdict::Fail => summary.failures += 1,
                Verdict::Blocked => summary.blocked += 1,
            }
        }
        summary
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

pub fn print_backtest_validation_report(report: &BacktestValidationReport) {
    println!("# Backtest Validation: {}", report.result_set.result_set_id);
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
        ],
    );
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
        &["verdict", "category", "question", "message"],
        &report
            .checks
            .iter()
            .map(|check| {
                vec![
                    check.verdict.as_str().to_string(),
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
