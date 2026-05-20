use eyre::Result;
use sqlx::PgPool;

use super::super::db::ResultSetRecord;
use super::super::report::{CheckResult, Verdict};
use super::common::count_check;

pub(super) async fn execution_delay_check(
    pool: &PgPool,
    result_set: &ResultSetRecord,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "terminal_report_matches_execution_delay",
        Verdict::Fail,
        "terminal execution reports land exactly submitted_block + configured execution_delay_blocks",
        "orders whose terminal report block does not match configured execution delay",
        r#"
        WITH cfg AS (
            SELECT COALESCE(NULLIF(config->>'execution_delay_blocks', '')::bigint, 1) AS delay_blocks
            FROM alpha_trading.backtest_result_sets
            WHERE result_set_id = $1
        ),
        order_events AS (
            SELECT t.trade_id,
                   te.order_id,
                   te.order_side,
                   MIN(te.block_number) FILTER (WHERE te.status = 'submitted') AS submitted_block,
                   MIN(te.block_number) FILTER (WHERE te.status IN ('confirmed', 'failed', 'cancelled')) AS terminal_block
            FROM alpha_trading.trades t
            JOIN alpha_trading.trade_events te ON te.trade_id = t.trade_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
            GROUP BY t.trade_id, te.order_id, te.order_side
        )
        SELECT count(*)
        FROM order_events, cfg
        WHERE submitted_block IS NOT NULL
          AND terminal_block IS NOT NULL
          AND terminal_block <> submitted_block + cfg.delay_blocks
        "#,
        &result_set.result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn confirmed_reports_have_simulated_outputs_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "confirmed_reports_have_simulation_outputs",
        Verdict::Fail,
        "confirmed execution reports retain EVM simulation outputs, gas, and buy token amount",
        "confirmed reports missing filled amount, gas, or buy token amount",
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_events te
        JOIN alpha_trading.trades t ON t.trade_id = te.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND te.status = 'confirmed'
          AND (
              nullif(te.filled_amount_raw, '') IS NULL
              OR te.filled_amount_decimals IS NULL
              OR te.gas_used IS NULL
              OR nullif(te.gas_cost_eth, '') IS NULL
              OR (
                  te.order_side = 'buy'
                  AND nullif(te.payload->'token_amount'->>'raw', '') IS NULL
              )
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn execution_replay_inputs_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "closed_trade_replay_inputs_present",
        Verdict::Fail,
        "closed trades retain enough order/report data for independent execution replay",
        "closed trades missing buy token amount, sell amount, or sell fill data",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades t
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND t.state = 'sell_confirmed'
          AND (
              t.entry_order_id IS NULL
              OR t.exit_order_id IS NULL
              OR NOT EXISTS (
                  SELECT 1
                  FROM alpha_trading.trade_events te
                  WHERE te.trade_id = t.trade_id
                    AND te.event_type = 'buy_confirmed'
                    AND te.payload->'token_amount'->>'raw' IS NOT NULL
              )
              OR NOT EXISTS (
                  SELECT 1
                  FROM alpha_trading.order_intents oi
                  WHERE oi.trade_id = t.trade_id
                    AND oi.side = 'sell'
                    AND nullif(oi.amount_raw, '') IS NOT NULL
              )
              OR NOT EXISTS (
                  SELECT 1
                  FROM alpha_trading.trade_events te
                  WHERE te.trade_id = t.trade_id
                    AND te.event_type = 'sell_confirmed'
                    AND nullif(te.filled_amount_raw, '') IS NOT NULL
              )
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}
