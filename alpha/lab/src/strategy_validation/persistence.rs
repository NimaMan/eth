use eyre::{Context, Result};
use sqlx::PgPool;

use super::report::{StrategyValidationReport, Verdict};

const VALIDATOR_VERSION: &str = concat!(
    "eth_alpha_lab-strategy-validation-",
    env!("CARGO_PKG_VERSION")
);

pub async fn persist_strategy_validation_report(
    pool: &PgPool,
    report: &StrategyValidationReport,
) -> Result<String> {
    ensure_validation_schema(pool).await?;
    let validation_id = validation_id(report);
    let report_json =
        serde_json::to_value(report).wrap_err("failed to encode validation report as json")?;
    let overall_verdict = overall_verdict(report).as_str();
    let status = if matches!(overall_verdict, "fail" | "blocked") {
        "needs_review"
    } else {
        "validated"
    };

    sqlx::query(
        r#"
        INSERT INTO alpha_trading.strategy_validation_reports (
            validation_id,
            result_set_id,
            strategy_name,
            profile,
            status,
            passed,
            warnings,
            failures,
            blocked,
            overall_verdict,
            report,
            validator_version
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (validation_id) DO UPDATE SET
            result_set_id = EXCLUDED.result_set_id,
            strategy_name = EXCLUDED.strategy_name,
            profile = EXCLUDED.profile,
            status = EXCLUDED.status,
            passed = EXCLUDED.passed,
            warnings = EXCLUDED.warnings,
            failures = EXCLUDED.failures,
            blocked = EXCLUDED.blocked,
            overall_verdict = EXCLUDED.overall_verdict,
            report = EXCLUDED.report,
            validator_version = EXCLUDED.validator_version,
            created_at = now()
        "#,
    )
    .bind(&validation_id)
    .bind(&report.result_set.result_set_id)
    .bind(report.strategy_filter.as_deref())
    .bind(&report.profile)
    .bind(status)
    .bind(report.summary.passed as i64)
    .bind(report.summary.warnings as i64)
    .bind(report.summary.failures as i64)
    .bind(report.summary.blocked as i64)
    .bind(overall_verdict)
    .bind(report_json)
    .bind(VALIDATOR_VERSION)
    .execute(pool)
    .await
    .wrap_err("failed to persist validation report")?;

    sqlx::query(
        r#"
        DELETE FROM alpha_trading.strategy_validation_reports
        WHERE result_set_id = $1
          AND strategy_name IS NOT DISTINCT FROM $2
          AND validation_id <> $3
        "#,
    )
    .bind(&report.result_set.result_set_id)
    .bind(report.strategy_filter.as_deref())
    .bind(&validation_id)
    .execute(pool)
    .await
    .wrap_err("failed to remove superseded validation reports")?;

    Ok(validation_id)
}

async fn ensure_validation_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS alpha_trading.strategy_validation_reports (
            validation_id TEXT PRIMARY KEY,
            result_set_id TEXT NOT NULL REFERENCES alpha_trading.backtest_result_sets(result_set_id) ON DELETE CASCADE,
            strategy_name TEXT,
            profile TEXT NOT NULL,
            status TEXT NOT NULL,
            passed BIGINT NOT NULL,
            warnings BIGINT NOT NULL,
            failures BIGINT NOT NULL,
            blocked BIGINT NOT NULL,
            overall_verdict TEXT NOT NULL,
            report JSONB NOT NULL,
            validator_version TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )
        "#,
    )
    .execute(pool)
    .await
    .wrap_err("failed to create validation report table")?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS strategy_validation_reports_latest_idx
        ON alpha_trading.strategy_validation_reports (
            result_set_id,
            strategy_name,
            profile,
            created_at DESC
        )
        "#,
    )
    .execute(pool)
    .await
    .wrap_err("failed to create validation report latest index")?;

    let legacy_table: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('alpha_trading.backtest_validation_reports')::text")
            .fetch_one(pool)
            .await
            .wrap_err("failed to check legacy validation report table")?;
    if legacy_table.is_some() {
        sqlx::query(
            r#"
            INSERT INTO alpha_trading.strategy_validation_reports (
                validation_id,
                result_set_id,
                strategy_name,
                profile,
                status,
                passed,
                warnings,
                failures,
                blocked,
                overall_verdict,
                report,
                validator_version,
                created_at
            )
            SELECT validation_id,
                   result_set_id,
                   strategy_name,
                   profile,
                   status,
                   passed,
                   warnings,
                   failures,
                   blocked,
                   overall_verdict,
                   report,
                   validator_version,
                   created_at
            FROM alpha_trading.backtest_validation_reports
            ON CONFLICT (validation_id) DO NOTHING
            "#,
        )
        .execute(pool)
        .await
        .wrap_err("failed to copy legacy validation reports")?;
    }

    Ok(())
}

fn overall_verdict(report: &StrategyValidationReport) -> Verdict {
    if report.summary.failures > 0 {
        Verdict::Fail
    } else if report.summary.blocked > 0 {
        Verdict::Blocked
    } else if report.summary.warnings > 0 {
        Verdict::Warn
    } else {
        Verdict::Pass
    }
}

fn validation_id(report: &StrategyValidationReport) -> String {
    let strategy = report.strategy_filter.as_deref().unwrap_or("all");
    format!(
        "val_{}_{}_{}",
        sanitize_id(&report.result_set.result_set_id),
        sanitize_id(strategy),
        sanitize_id(&report.profile)
    )
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
