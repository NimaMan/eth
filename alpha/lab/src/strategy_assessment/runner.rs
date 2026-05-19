use eyre::Result;
use serde_json::json;
use sqlx::PgPool;

use crate::backtest_validation::db::{load_result_set, load_strategy_summaries};

use super::db::{load_assessment_metrics, AssessmentMetrics};
use super::report::{
    AssessmentQuestion, AssessmentStatus, StrategyAssessmentReport, StrategyAssessmentSummary,
};

#[derive(Clone, Debug)]
pub struct StrategyAssessmentOptions {
    pub result_set_id: String,
    pub strategy: Option<String>,
}

pub async fn assess_strategy(
    pool: &PgPool,
    options: StrategyAssessmentOptions,
) -> Result<StrategyAssessmentReport> {
    let result_set = load_result_set(pool, &options.result_set_id).await?;
    let strategy_summaries =
        load_strategy_summaries(pool, &options.result_set_id, options.strategy.as_deref()).await?;
    let metrics =
        load_assessment_metrics(pool, &options.result_set_id, options.strategy.as_deref()).await?;
    let summary = StrategyAssessmentSummary {
        trade_count: metrics.trade_count,
        closed_trades: metrics.closed_trades,
        open_trades: metrics.open_trades,
        failed_trades: metrics.failed_trades,
        exposure_trades: metrics.exposure_trades,
        failed_exit_exposure_trades: metrics.failed_exit_exposure_trades,
        winner_count: metrics.winner_count,
        loser_count: metrics.loser_count,
        total_entry_cost_eth: metrics.total_entry_cost_eth,
        realized_pnl_eth: metrics.realized_pnl_eth,
        unrealized_pnl_eth: metrics.unrealized_pnl_eth,
        total_pnl_eth: metrics.total_pnl_eth,
        gross_profit_eth: metrics.gross_profit_eth,
        gross_loss_eth: metrics.gross_loss_eth,
        profit_factor: metrics.profit_factor,
        top5_share_of_net_pnl_percent: metrics.top5_share_of_net_pnl_percent,
        pnl_ex_top5_eth: metrics.pnl_ex_top5_eth,
        top5_loser_share_of_gross_loss_percent: metrics.top5_loser_share_of_gross_loss_percent,
        exposure_to_capital_percent: metrics.exposure_to_capital_percent,
        exposure_unrealized_to_net_pnl_percent: metrics.exposure_unrealized_to_net_pnl_percent,
    };

    Ok(StrategyAssessmentReport {
        result_set,
        strategy_filter: options.strategy,
        summary,
        strategy_summaries,
        questions: assessment_questions(&metrics),
        procedure: assessment_procedure(),
    })
}

fn assessment_questions(metrics: &AssessmentMetrics) -> Vec<AssessmentQuestion> {
    vec![
        top_winner_concentration(metrics),
        profit_factor_question(metrics),
        loss_tail_question(metrics),
        open_exposure_question(metrics),
        failed_exit_question(metrics),
    ]
}

fn top_winner_concentration(metrics: &AssessmentMetrics) -> AssessmentQuestion {
    let assessment = if metrics.trade_count == 0 {
        AssessmentStatus::InsufficientData
    } else if metrics.total_pnl_eth <= 0.0 {
        AssessmentStatus::Review
    } else if metrics.pnl_ex_top5_eth < 0.0
        || metrics.top5_share_of_net_pnl_percent.unwrap_or_default() > 100.0
    {
        AssessmentStatus::Fragile
    } else if metrics.pnl_ex_top1_eth < 0.0
        || metrics.top5_share_of_net_pnl_percent.unwrap_or_default() > 60.0
    {
        AssessmentStatus::Review
    } else {
        AssessmentStatus::Stable
    };
    let answer = match assessment {
        AssessmentStatus::InsufficientData => "no trades were available for this scope".to_string(),
        AssessmentStatus::Fragile => format!(
            "top five winners are {:.2}% of net PnL and PnL excluding them is {:.6} ETH",
            metrics.top5_share_of_net_pnl_percent.unwrap_or_default(),
            metrics.pnl_ex_top5_eth
        ),
        AssessmentStatus::Review if metrics.total_pnl_eth <= 0.0 => format!(
            "net PnL is {:.6} ETH, so there is no positive conclusion to stress",
            metrics.total_pnl_eth
        ),
        AssessmentStatus::Review => format!(
            "winner concentration is material: top five are {:.2}% of net PnL",
            metrics.top5_share_of_net_pnl_percent.unwrap_or_default()
        ),
        AssessmentStatus::Stable => format!(
            "PnL remains {:.6} ETH after removing the top five winners",
            metrics.pnl_ex_top5_eth
        ),
    };
    question(
        "distribution",
        "top_winner_concentration",
        "Does the positive PnL conclusion survive removing the biggest winners?",
        "A strategy can look profitable because a small number of trades overwhelm many losers.",
        "Compute top1/top5/top10 positive-trade PnL, divide by net PnL, and recompute net PnL after removing those winners.",
        assessment,
        answer,
        json!({
            "total_pnl_eth": metrics.total_pnl_eth,
            "top1_winner_pnl_eth": metrics.top1_winner_pnl_eth,
            "top5_winner_pnl_eth": metrics.top5_winner_pnl_eth,
            "top10_winner_pnl_eth": metrics.top10_winner_pnl_eth,
            "pnl_ex_top1_eth": metrics.pnl_ex_top1_eth,
            "pnl_ex_top5_eth": metrics.pnl_ex_top5_eth,
            "pnl_ex_top10_eth": metrics.pnl_ex_top10_eth,
            "top1_share_of_net_pnl_percent": metrics.top1_share_of_net_pnl_percent,
            "top5_share_of_net_pnl_percent": metrics.top5_share_of_net_pnl_percent,
            "top10_share_of_net_pnl_percent": metrics.top10_share_of_net_pnl_percent,
            "review_threshold_percent": 60.0,
            "fragile_threshold_percent": 100.0,
        }),
    )
}

fn profit_factor_question(metrics: &AssessmentMetrics) -> AssessmentQuestion {
    let profit_factor = metrics.profit_factor.unwrap_or(f64::INFINITY);
    let assessment = if metrics.trade_count == 0 || metrics.gross_profit_eth == 0.0 {
        AssessmentStatus::InsufficientData
    } else if metrics.gross_loss_eth == 0.0 {
        AssessmentStatus::Stable
    } else if metrics.total_pnl_eth > 0.0 && profit_factor < 1.2 {
        AssessmentStatus::Fragile
    } else if metrics.total_pnl_eth > 0.0 && profit_factor < 1.5 {
        AssessmentStatus::Review
    } else if metrics.total_pnl_eth <= 0.0 {
        AssessmentStatus::Review
    } else {
        AssessmentStatus::Stable
    };
    let answer = match assessment {
        AssessmentStatus::InsufficientData => "gross profit is zero or no trades exist".to_string(),
        AssessmentStatus::Stable if metrics.gross_loss_eth == 0.0 => {
            "gross loss is zero for this scope".to_string()
        }
        AssessmentStatus::Fragile => format!("profit factor is only {profit_factor:.4}"),
        AssessmentStatus::Review if metrics.total_pnl_eth <= 0.0 => {
            format!("net PnL is {:.6} ETH", metrics.total_pnl_eth)
        }
        AssessmentStatus::Review => format!("profit factor is {profit_factor:.4}"),
        AssessmentStatus::Stable => format!("profit factor is {profit_factor:.4}"),
    };
    question(
        "distribution",
        "profit_factor",
        "Is gross profit large enough to absorb gross losses?",
        "Net PnL hides whether many losing trades are nearly consuming all winners.",
        "Divide gross positive PnL by absolute gross negative PnL.",
        assessment,
        answer,
        json!({
            "gross_profit_eth": metrics.gross_profit_eth,
            "gross_loss_eth": metrics.gross_loss_eth,
            "profit_factor": metrics.profit_factor,
            "review_threshold": 1.5,
            "fragile_threshold": 1.2,
        }),
    )
}

fn loss_tail_question(metrics: &AssessmentMetrics) -> AssessmentQuestion {
    let share = metrics
        .top5_loser_share_of_gross_loss_percent
        .unwrap_or_default();
    let assessment = if metrics.loser_count == 0 {
        AssessmentStatus::InsufficientData
    } else if share >= 70.0 && metrics.loser_count >= 5 {
        AssessmentStatus::Review
    } else {
        AssessmentStatus::Stable
    };
    let answer = match assessment {
        AssessmentStatus::InsufficientData => "there are no losing trades in this scope".to_string(),
        AssessmentStatus::Review => format!(
            "top five losers explain {:.2}% of gross loss; inspect these trades for avoidable signals",
            share
        ),
        AssessmentStatus::Stable | AssessmentStatus::Fragile => {
            format!("top five losers explain {:.2}% of gross loss", share)
        }
    };
    question(
        "losses",
        "loss_tail_concentration",
        "Are losses concentrated in a small tail we can explain or avoid?",
        "Large isolated losses are often tied to specific risk signals, execution timing, or pool conditions.",
        "Rank negative-PnL trades by loss size and divide the top-five absolute loss by total gross loss.",
        assessment,
        answer,
        json!({
            "loser_count": metrics.loser_count,
            "gross_loss_eth": metrics.gross_loss_eth,
            "top5_loser_loss_eth": metrics.top5_loser_loss_eth,
            "top5_loser_share_of_gross_loss_percent": metrics.top5_loser_share_of_gross_loss_percent,
            "review_threshold_percent": 70.0,
        }),
    )
}

fn open_exposure_question(metrics: &AssessmentMetrics) -> AssessmentQuestion {
    let exposure_share = metrics.exposure_to_capital_percent.unwrap_or_default();
    let unrealized_share = metrics
        .exposure_unrealized_to_net_pnl_percent
        .map(f64::abs)
        .unwrap_or_default();
    let assessment = if metrics.trade_count == 0 {
        AssessmentStatus::InsufficientData
    } else if exposure_share > 35.0 || unrealized_share > 50.0 {
        AssessmentStatus::Fragile
    } else if exposure_share > 20.0 || unrealized_share > 25.0 {
        AssessmentStatus::Review
    } else {
        AssessmentStatus::Stable
    };
    let answer = match assessment {
        AssessmentStatus::InsufficientData => "no trades were available for this scope".to_string(),
        AssessmentStatus::Fragile => format!(
            "open exposure is {:.2}% of deployed capital and unrealized exposure is {:.2}% of net PnL",
            exposure_share, unrealized_share
        ),
        AssessmentStatus::Review => format!(
            "open exposure is material at {:.2}% of deployed capital",
            exposure_share
        ),
        AssessmentStatus::Stable => format!(
            "open exposure is {:.2}% of deployed capital",
            exposure_share
        ),
    };
    question(
        "exposure",
        "open_exposure_materiality",
        "Does unresolved exposure materially affect the strategy conclusion?",
        "Open positions and failed exits can move after the report, so realized-only conclusions may be misleading.",
        "Sum entry cost and unrealized PnL for active exposure states, then divide by total deployed capital and net PnL.",
        assessment,
        answer,
        json!({
            "exposure_trades": metrics.exposure_trades,
            "exposure_entry_cost_eth": metrics.exposure_entry_cost_eth,
            "exposure_current_value_eth": metrics.exposure_current_value_eth,
            "exposure_unrealized_pnl_eth": metrics.exposure_unrealized_pnl_eth,
            "exposure_drawdown_eth": metrics.exposure_drawdown_eth,
            "exposure_to_capital_percent": metrics.exposure_to_capital_percent,
            "exposure_unrealized_to_net_pnl_percent": metrics.exposure_unrealized_to_net_pnl_percent,
            "review_capital_threshold_percent": 20.0,
            "fragile_capital_threshold_percent": 35.0,
        }),
    )
}

fn failed_exit_question(metrics: &AssessmentMetrics) -> AssessmentQuestion {
    let failed_exit_rate = if metrics.exposure_trades > 0 {
        metrics.failed_exit_exposure_trades as f64 / metrics.exposure_trades as f64 * 100.0
    } else {
        0.0
    };
    let assessment = if metrics.trade_count == 0 {
        AssessmentStatus::InsufficientData
    } else if failed_exit_rate > 10.0 {
        AssessmentStatus::Fragile
    } else if metrics.failed_exit_exposure_trades > 0 {
        AssessmentStatus::Review
    } else {
        AssessmentStatus::Stable
    };
    let answer = match assessment {
        AssessmentStatus::InsufficientData => "no trades were available for this scope".to_string(),
        AssessmentStatus::Fragile => format!(
            "{} failed-exit exposure trades ({failed_exit_rate:.2}% of exposure)",
            metrics.failed_exit_exposure_trades
        ),
        AssessmentStatus::Review => format!(
            "{} failed-exit exposure trades need inspection",
            metrics.failed_exit_exposure_trades
        ),
        AssessmentStatus::Stable => "no failed-exit exposure remains".to_string(),
    };
    question(
        "exposure",
        "failed_exit_exposure",
        "Are failed exits leaving unresolved exposure?",
        "A strategy with profitable entries can still be unusable if exits fail or are cancelled often.",
        "Count sell_failed and sell_cancelled trades that remain in exposure states and divide by active exposure trades.",
        assessment,
        answer,
        json!({
            "exposure_trades": metrics.exposure_trades,
            "failed_exit_exposure_trades": metrics.failed_exit_exposure_trades,
            "failed_exit_rate_percent": failed_exit_rate,
            "fragile_threshold_percent": 10.0,
        }),
    )
}

fn question(
    category: impl Into<String>,
    code: impl Into<String>,
    question_text: impl Into<String>,
    why: impl Into<String>,
    quantification: impl Into<String>,
    assessment: AssessmentStatus,
    answer: impl Into<String>,
    evidence: serde_json::Value,
) -> AssessmentQuestion {
    AssessmentQuestion {
        category: category.into(),
        code: code.into(),
        question: question_text.into(),
        why: why.into(),
        quantification: quantification.into(),
        assessment,
        answer: answer.into(),
        evidence,
    }
}

fn assessment_procedure() -> Vec<String> {
    vec![
        "Load result-set metadata and scoped strategy trade rows from alpha_trading.".to_string(),
        "Aggregate persisted trade PnL, exposure, winner, and loser metrics without changing validation verdicts.".to_string(),
        "Answer explicit strategy-quality questions using documented formulas and thresholds.".to_string(),
        "Return backend-owned assessment results for the frontend to render without recalculating distribution or exposure logic.".to_string(),
    ]
}
