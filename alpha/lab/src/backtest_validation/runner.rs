use clap::ValueEnum;
use eyre::Result;
use sqlx::PgPool;
use std::fmt;

use super::checks::run_checks;
use super::db::{load_result_set, load_strategy_summaries, load_trade_samples};
use super::report::{BacktestValidationReport, ValidationSummary};

#[derive(Clone, Debug)]
pub struct ValidationOptions {
    pub result_set_id: String,
    pub strategy: Option<String>,
    pub profile: ValidationProfile,
    pub sample_limit: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ValidationProfile {
    Quick,
    Standard,
    Strict,
}

impl ValidationProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Strict => "strict",
        }
    }

    pub fn default_sample_limit(self) -> i64 {
        match self {
            Self::Quick => 3,
            Self::Standard => 10,
            Self::Strict => 25,
        }
    }
}

impl fmt::Display for ValidationProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
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
        options.sample_limit.max(1),
    )
    .await?;
    let summary = ValidationSummary::from_checks(&checks);

    Ok(BacktestValidationReport {
        result_set,
        strategy_filter: options.strategy,
        profile: options.profile.as_str().to_string(),
        summary,
        strategy_summaries,
        checks,
        samples,
        procedure: validation_procedure(options.profile),
    })
}

fn validation_procedure(profile: ValidationProfile) -> Vec<String> {
    let mut steps = vec![
        "Load the result set, run provenance, strategy config, and scoped trade rows from alpha_trading.".to_string(),
        "Validate result-set metadata and signal scope before reading any PnL as meaningful.".to_string(),
        "Validate trade lifecycle ordering from trade_events: buy submitted, buy confirmed, sell submitted, sell confirmed.".to_string(),
        "Validate decision timing by requiring submitted trade events to have same-block strategy_decisions and risk exits to have prior local risk_events.".to_string(),
        "Validate accounting from persisted trade fields: total PnL equals realized plus unrealized, and closed realized PnL equals exit value minus entry cost minus gas.".to_string(),
        "Validate snapshot consistency: latest_snapshot_block must match trade_snapshots and closed trades should have a final sell_confirmed snapshot.".to_string(),
        "Validate replay readiness by ensuring closed trades retain buy token amount, sell order amount, and sell filled amount for independent chain-sim replay.".to_string(),
        "Sample top winners and worst losers for mandatory follow-up forensic replay.".to_string(),
    ];
    if matches!(
        profile,
        ValidationProfile::Standard | ValidationProfile::Strict
    ) {
        steps.push(
            "Use a wider top/worst sample so concentration and tail failures are reviewed before accepting strategy-level conclusions."
                .to_string(),
        );
    }
    if matches!(profile, ValidationProfile::Strict) {
        steps.push(
            "Strict profile is reserved for adding EVM execution replay over the sampled closed trades; DB checks must pass first."
                .to_string(),
        );
    }
    steps
}
