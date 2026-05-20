use eyre::Result;
use sqlx::PgPool;

use super::super::report::{CheckResult, Verdict};
use super::common::count_check;

pub(super) async fn submitted_events_have_decisions_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
    side: &str,
) -> Result<CheckResult> {
    let (event_type, action, code) = match side {
        "buy" => ("buy_submitted", "submit_buy", "buy_submitted_has_decision"),
        "sell" => (
            "sell_submitted",
            "submit_sell",
            "sell_submitted_has_decision",
        ),
        _ => unreachable!("unsupported side"),
    };
    let sql = format!(
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_events te
        JOIN alpha_trading.trades t ON t.trade_id = te.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND te.event_type = '{event_type}'
          AND NOT EXISTS (
              SELECT 1
              FROM alpha_trading.strategy_decisions sd
              WHERE sd.run_id = t.run_id
                AND sd.strategy_name = t.strategy_name
                AND sd.token_address = t.token_address
                AND sd.pool_address = t.pool_address
                AND sd.block_number = te.block_number
                AND sd.action = '{action}'
          )
        "#
    );
    count_check(
        pool,
        "decision_timing",
        code,
        Verdict::Fail,
        format!("every {event_type} event has a same-block `{action}` decision"),
        format!("{event_type} events without a matching decision"),
        &sql,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn risk_sell_decisions_have_prior_risk_events_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "decision_timing",
        "risk_sell_has_available_signal",
        Verdict::Fail,
        "risk sell decisions have a risk event at or before the decision block",
        "risk sell decisions without prior local risk evidence",
        r#"
        SELECT count(*)
        FROM alpha_trading.strategy_decisions sd
        JOIN alpha_trading.backtest_result_set_runs rsr ON rsr.run_id = sd.run_id
        WHERE rsr.result_set_id = $1
          AND ($2::text IS NULL OR sd.strategy_name = $2)
          AND sd.action = 'submit_sell'
          AND sd.event_source IN ('risk', 'mempool_signal', 'historical_mempool_signal', 'risk_atlas_mined_chain')
          AND NOT EXISTS (
              SELECT 1
              FROM alpha_trading.risk_events re
              WHERE re.run_id = sd.run_id
                AND lower(re.token_address) = lower(sd.token_address)
                AND (re.pool_address IS NULL OR sd.pool_address IS NULL OR lower(re.pool_address) = lower(sd.pool_address))
                AND re.observed_block <= sd.block_number
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn risk_sell_decisions_submit_on_signal_block_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "decision_timing",
        "risk_sell_signal_block_immediate",
        Verdict::Fail,
        "risk-triggered sell decisions are submitted on the same block as the matching risk signal",
        "risk sell decisions whose matching risk evidence is not same-block",
        r#"
        SELECT count(*)
        FROM alpha_trading.strategy_decisions sd
        JOIN alpha_trading.backtest_result_set_runs rsr ON rsr.run_id = sd.run_id
        WHERE rsr.result_set_id = $1
          AND ($2::text IS NULL OR sd.strategy_name = $2)
          AND sd.action = 'submit_sell'
          AND sd.event_source IN ('risk', 'mempool_signal', 'historical_mempool_signal', 'risk_atlas_mined_chain')
          AND NOT EXISTS (
              SELECT 1
              FROM alpha_trading.risk_events re
              WHERE re.run_id = sd.run_id
                AND lower(re.token_address) = lower(sd.token_address)
                AND (re.pool_address IS NULL OR sd.pool_address IS NULL OR lower(re.pool_address) = lower(sd.pool_address))
                AND re.observed_block = sd.block_number
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}
