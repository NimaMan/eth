use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::render;
use crate::strategy_validation::db::{ResultSetRecord, StrategySummary};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyAssessmentReport {
    pub result_set: ResultSetRecord,
    pub strategy_filter: Option<String>,
    pub summary: StrategyAssessmentSummary,
    pub strategy_summaries: Vec<StrategySummary>,
    pub questions: Vec<AssessmentQuestion>,
    pub procedure: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyAssessmentSummary {
    pub trade_count: i64,
    pub closed_trades: i64,
    pub open_trades: i64,
    pub failed_trades: i64,
    pub exposure_trades: i64,
    pub failed_exit_exposure_trades: i64,
    pub winner_count: i64,
    pub loser_count: i64,
    pub total_entry_cost_eth: f64,
    pub realized_pnl_eth: f64,
    pub unrealized_pnl_eth: f64,
    pub total_pnl_eth: f64,
    pub gross_profit_eth: f64,
    pub gross_loss_eth: f64,
    pub profit_factor: Option<f64>,
    pub top5_share_of_net_pnl_percent: Option<f64>,
    pub pnl_ex_top5_eth: f64,
    pub top5_loser_share_of_gross_loss_percent: Option<f64>,
    pub exposure_to_capital_percent: Option<f64>,
    pub exposure_unrealized_to_net_pnl_percent: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssessmentQuestion {
    pub category: String,
    pub code: String,
    pub question: String,
    pub why: String,
    pub quantification: String,
    pub assessment: AssessmentStatus,
    pub answer: String,
    pub evidence: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    Stable,
    Review,
    Fragile,
    InsufficientData,
}

impl AssessmentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Review => "review",
            Self::Fragile => "fragile",
            Self::InsufficientData => "insufficient_data",
        }
    }
}

pub fn print_strategy_assessment_report(report: &StrategyAssessmentReport) {
    println!("# Strategy Assessment: {}", report.result_set.result_set_id);
    println!();
    println!(
        "- strategy filter: `{}`",
        report.strategy_filter.as_deref().unwrap_or("<all>")
    );
    println!(
        "- mode/status: `{}` / `{}`",
        report.result_set.mode, report.result_set.status
    );
    println!();

    println!("## Summary");
    render::print_table(
        &["metric", "value"],
        &[
            vec!["trades".to_string(), report.summary.trade_count.to_string()],
            vec![
                "closed".to_string(),
                report.summary.closed_trades.to_string(),
            ],
            vec!["open".to_string(), report.summary.open_trades.to_string()],
            vec![
                "failed".to_string(),
                report.summary.failed_trades.to_string(),
            ],
            vec![
                "total PnL ETH".to_string(),
                format!("{:.6}", report.summary.total_pnl_eth),
            ],
            vec![
                "profit factor".to_string(),
                fmt_opt(report.summary.profit_factor),
            ],
            vec![
                "top5/net PnL".to_string(),
                fmt_pct(report.summary.top5_share_of_net_pnl_percent),
            ],
            vec![
                "PnL ex top5".to_string(),
                format!("{:.6}", report.summary.pnl_ex_top5_eth),
            ],
            vec![
                "exposure/capital".to_string(),
                fmt_pct(report.summary.exposure_to_capital_percent),
            ],
        ],
    );
    println!();

    println!("## Questions");
    render::print_table(
        &["assessment", "category", "question", "answer"],
        &report
            .questions
            .iter()
            .map(|question| {
                vec![
                    question.assessment.as_str().to_string(),
                    question.category.clone(),
                    question.question.clone(),
                    question.answer.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    );
}

fn fmt_opt(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_pct(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}%"))
        .unwrap_or_else(|| "-".to_string())
}
