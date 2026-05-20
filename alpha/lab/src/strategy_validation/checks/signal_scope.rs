use eyre::Result;
use serde_json::json;
use sqlx::PgPool;

use super::super::db::ResultSetRecord;
use super::super::report::{CheckResult, Verdict};
use super::common::{check, count_check};

pub(super) async fn historical_mempool_scope_check(
    pool: &PgPool,
    result_set: &ResultSetRecord,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    if result_set.mode != "historical" {
        return Ok(check(
            "signal_scope",
            "historical_mempool_rows",
            Verdict::Pass,
            "non-historical result set is not checked for historical mempool exclusion",
            json!({ "mode": result_set.mode }),
        ));
    }
    count_check(
        pool,
        "signal_scope",
        "historical_mempool_rows",
        Verdict::Fail,
        "historical result set contains no pending mempool risk rows",
        "pending mempool risk rows joined to historical result set",
        r#"
        SELECT count(*)
        FROM alpha_trading.risk_events re
        JOIN alpha_trading.backtest_result_set_runs rsr ON rsr.run_id = re.run_id
        WHERE rsr.result_set_id = $1
          AND re.pending_tx_hash IS NOT NULL
          AND (
              $2::text IS NULL
              OR EXISTS (
                  SELECT 1
                  FROM alpha_trading.trades t
                  WHERE t.result_set_id = rsr.result_set_id
                    AND t.run_id = re.run_id
                    AND t.strategy_name = $2
              )
          )
        "#,
        &result_set.result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn historical_market_buy_decisions_have_observations_check(
    pool: &PgPool,
    result_set: &ResultSetRecord,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    if result_set.mode != "historical" {
        return Ok(check(
            "signal_scope",
            "market_buy_has_historical_observation",
            Verdict::Pass,
            "non-historical result set is not checked against Risk Atlas replay observations",
            json!({ "mode": result_set.mode }),
        ));
    }
    count_check(
        pool,
        "signal_scope",
        "market_buy_has_historical_observation",
        Verdict::Fail,
        "historical market buy decisions join to the replay observation for that token, pool, and block",
        "historical market buy decisions without matching replay observation rows",
        r#"
        WITH rs AS (
            SELECT config->>'replay_run_id' AS replay_run_id
            FROM alpha_trading.backtest_result_sets
            WHERE result_set_id = $1
        )
        SELECT count(*)
        FROM alpha_trading.strategy_decisions sd
        JOIN alpha_trading.backtest_result_set_runs rsr ON rsr.run_id = sd.run_id
        CROSS JOIN rs
        WHERE rsr.result_set_id = $1
          AND ($2::text IS NULL OR sd.strategy_name = $2)
          AND sd.action = 'submit_buy'
          AND sd.event_source = 'market'
          AND rs.replay_run_id IS NOT NULL
          AND NOT EXISTS (
              SELECT 1
              FROM risk_atlas_observations o
              WHERE o.run_id = rs.replay_run_id
                AND lower(o.token_address) = lower(sd.token_address)
                AND lower(o.pool_address) = lower(COALESCE(NULLIF(split_part(sd.pool_address, ':', 2), ''), sd.pool_address))
                AND o.block_number = sd.block_number
          )
        "#,
        &result_set.result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn submit_decisions_in_range_check(
    pool: &PgPool,
    result_set: &ResultSetRecord,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    if result_set.start_block.is_none() || result_set.end_block.is_none() {
        return Ok(check(
            "signal_scope",
            "submit_decisions_within_result_range",
            Verdict::Pass,
            "result set has no fixed block range to enforce",
            json!({
                "start_block": result_set.start_block,
                "end_block": result_set.end_block,
            }),
        ));
    }
    count_check(
        pool,
        "signal_scope",
        "submit_decisions_within_result_range",
        Verdict::Fail,
        "submitted decisions are made inside the result-set input block range",
        "submitted decisions outside the result-set block range",
        r#"
        SELECT count(*)
        FROM alpha_trading.strategy_decisions sd
        JOIN alpha_trading.backtest_result_set_runs rsr ON rsr.run_id = sd.run_id
        JOIN alpha_trading.backtest_result_sets rs ON rs.result_set_id = rsr.result_set_id
        WHERE rsr.result_set_id = $1
          AND ($2::text IS NULL OR sd.strategy_name = $2)
          AND sd.action IN ('submit_buy', 'submit_sell')
          AND (
              sd.block_number IS NULL
              OR sd.block_number < rs.start_block
              OR sd.block_number > rs.end_block
          )
        "#,
        &result_set.result_set_id,
        strategy,
    )
    .await
}
