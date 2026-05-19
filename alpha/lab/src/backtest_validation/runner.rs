use eyre::Result;
use sqlx::PgPool;

use super::checks::run_checks;
use super::db::{load_result_set, load_strategy_summaries, load_trade_samples};
use super::report::{BacktestValidationReport, ValidationSummary};

pub const COMPREHENSIVE_VALIDATION_PROFILE: &str = "comprehensive";
const COMPREHENSIVE_SAMPLE_LIMIT: i64 = 25;

#[derive(Clone, Debug)]
pub struct ValidationOptions {
    pub result_set_id: String,
    pub strategy: Option<String>,
}

pub async fn validate_backtest(
    pool: &PgPool,
    options: ValidationOptions,
) -> Result<BacktestValidationReport> {
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

    Ok(BacktestValidationReport {
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
        "Validate trade lifecycle ordering from trade_events: buy submitted, buy confirmed, sell submitted, sell confirmed.".to_string(),
        "Validate decision timing by requiring submitted trade events to have same-block strategy_decisions, historical buys to join replay observations, risk exits to use same-block local risk_events, and submitted decisions to stay inside the replayed range.".to_string(),
        "Validate the execution model: terminal execution reports must land at submitted_block + execution_delay_blocks and confirmed fills must retain EVM simulation outputs, gas, and buy token amounts.".to_string(),
        "Validate accounting from persisted simulation fills: entry cost equals buy fill, exit value equals sell fill, total PnL equals realized plus unrealized, and closed realized PnL equals exit value minus entry cost minus gas.".to_string(),
        "Validate snapshot consistency: closed trades must not receive later snapshots, open-state valuations must not pass the sell block, latest_snapshot_block and latest PnL values must match trade_snapshots, and closed trades should have a final sell_confirmed snapshot.".to_string(),
        "Validate replay readiness by ensuring closed trades retain buy token amount, sell order amount, and sell filled amount for independent chain-sim replay.".to_string(),
        "Sample top winners and worst losers with the comprehensive sample size so concentration and tail failures are reviewed before accepting strategy-level conclusions.".to_string(),
    ]
}
