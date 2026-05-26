use eyre::Result;
use sqlx::PgPool;

use super::checks::run_checks;
use super::db::{load_result_set, load_strategy_summaries, load_trade_samples};
use super::report::{StrategyValidationReport, ValidationSummary};

pub const COMPREHENSIVE_VALIDATION_PROFILE: &str = "comprehensive";
const COMPREHENSIVE_SAMPLE_LIMIT: i64 = 25;

#[derive(Clone, Debug)]
pub struct ValidationOptions {
    pub result_set_id: String,
    pub strategy: Option<String>,
}

pub async fn validate_strategy(
    pool: &PgPool,
    options: ValidationOptions,
) -> Result<StrategyValidationReport> {
    let result_set = load_result_set(pool, &options.result_set_id).await?;
    let strategy_summaries =
        load_strategy_summaries(pool, &options.result_set_id, options.strategy.as_deref()).await?;
    let checks = run_checks(
        pool,
        &result_set,
        options.strategy.as_deref(),
        &strategy_summaries,
    )
    .await?;
    let samples = load_trade_samples(
        pool,
        &options.result_set_id,
        options.strategy.as_deref(),
        COMPREHENSIVE_SAMPLE_LIMIT,
    )
    .await?;
    let summary = ValidationSummary::from_checks(&checks);

    Ok(StrategyValidationReport {
        result_set,
        strategy_filter: options.strategy,
        profile: COMPREHENSIVE_VALIDATION_PROFILE.to_string(),
        summary,
        strategy_summaries,
        checks,
        samples,
        procedure: validation_procedure(),
    })
}

fn validation_procedure() -> Vec<String> {
    vec![
        "Load the result set, run provenance, strategy config, and scoped trade rows from alpha_trading.".to_string(),
        "Validate result-set metadata and signal scope before reading any PnL as meaningful.".to_string(),
        "Validate trade lifecycle ordering from trade_events and execution_reports: one submitted row per order, at most one terminal row, report/event mirror rows, buy submitted, buy confirmed, sell submitted, and sell confirmed.".to_string(),
        "Validate decision timing by requiring submitted trade events to have same-block strategy_decisions, historical buys to join replay observations, risk exits to use same-block local risk_events, and submitted decisions to stay inside the replayed range.".to_string(),
        "Validate the execution model: terminal execution reports must land at submitted_block + execution_delay_blocks and confirmed fills must retain EVM simulation outputs, gas, and buy token amounts.".to_string(),
        "Validate tail-entry coverage for strategies that depend on mempool trading_enabled signals: observed signals, mempool entry evidence, exact-vault eligible evidence, tail-entry intents, and execution outcomes must be counted explicitly.".to_string(),
        "Validate rollups and accounting from persisted events: trade rows must match position lifecycle fields, gas must equal the trade event gas sum, entry cost equals buy fill, exit value equals sell fill, total PnL equals realized plus unrealized, and closed realized PnL equals exit value minus entry cost minus gas.".to_string(),
        "Validate snapshot consistency: valuation snapshots must not precede buy confirmation, closed trades must not receive later snapshots, open-state valuations must not pass the sell block, latest snapshot block coordinates, PnL, and ROI must match trade_snapshots, and closed trades must have a terminal sell_confirmed latest snapshot.".to_string(),
        "Validate replay readiness by ensuring closed trades retain buy token amount, sell order amount, and sell filled amount for independent chain-sim replay.".to_string(),
        "Sample top winners and worst losers only as supporting evidence for debugging failed invariants; distribution quality is assessed by strategy_assessment, not validation.".to_string(),
    ]
}
