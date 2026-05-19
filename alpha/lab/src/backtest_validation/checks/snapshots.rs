use eyre::Result;
use sqlx::PgPool;

use super::super::report::{CheckResult, Verdict};
use super::common::count_check;

pub(super) async fn latest_snapshot_block_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "latest_snapshot_block_matches_snapshots",
        Verdict::Fail,
        "trade latest_snapshot_block matches max persisted trade snapshot block",
        "trades whose latest_snapshot_block differs from max snapshot block",
        r#"
        WITH latest AS (
            SELECT trade_id, max(block_number) AS max_snapshot_block
            FROM alpha_trading.trade_snapshots
            GROUP BY trade_id
        )
        SELECT count(*)
        FROM alpha_trading.trades t
        LEFT JOIN latest ON latest.trade_id = t.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND t.latest_snapshot_block IS DISTINCT FROM latest.max_snapshot_block
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn no_snapshots_after_sell_confirmed_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "no_snapshots_after_sell_confirmed",
        Verdict::Fail,
        "no trade snapshots are appended after a sell_confirmed snapshot",
        "closed trades with snapshots appended after sell_confirmed",
        r#"
        WITH terminal AS (
            SELECT ts.trade_id, min(ts.id) AS sell_snapshot_id
            FROM alpha_trading.trade_snapshots ts
            JOIN alpha_trading.trades t ON t.trade_id = ts.trade_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
              AND ts.state = 'sell_confirmed'
            GROUP BY ts.trade_id
        )
        SELECT count(DISTINCT ts.trade_id)
        FROM alpha_trading.trade_snapshots ts
        JOIN terminal ON terminal.trade_id = ts.trade_id
        WHERE ts.id > terminal.sell_snapshot_id
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn no_future_valued_open_snapshots_after_sell_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "no_open_snapshot_valued_after_sell_confirmed",
        Verdict::Fail,
        "open-state snapshots for closed trades are not valued after the sell_confirmed block",
        "closed trades with open-state snapshots valued after sell_confirmed",
        r#"
        WITH terminal AS (
            SELECT ts.trade_id, min(ts.block_number) AS sell_confirmed_block
            FROM alpha_trading.trade_snapshots ts
            JOIN alpha_trading.trades t ON t.trade_id = ts.trade_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
              AND ts.state = 'sell_confirmed'
            GROUP BY ts.trade_id
        )
        SELECT count(DISTINCT ts.trade_id)
        FROM alpha_trading.trade_snapshots ts
        JOIN terminal ON terminal.trade_id = ts.trade_id
        WHERE ts.state <> 'sell_confirmed'
          AND coalesce(ts.valuation_block_number, ts.block_number) > terminal.sell_confirmed_block
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn latest_snapshot_values_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "latest_snapshot_values_match_trade",
        Verdict::Fail,
        "trade latest snapshot fields match the latest persisted trade snapshot",
        "trades whose latest snapshot fields differ from latest snapshot values",
        r#"
        WITH latest AS (
            SELECT DISTINCT ON (trade_id)
                   trade_id,
                   block_number,
                   observed_block_number,
                   valuation_block_number,
                   current_value_eth,
                   realized_pnl_eth,
                   unrealized_pnl_eth,
                   total_pnl_eth,
                   roi
            FROM alpha_trading.trade_snapshots
            ORDER BY trade_id, block_number DESC NULLS LAST, id DESC
        )
        SELECT count(*)
        FROM alpha_trading.trades t
        JOIN latest ON latest.trade_id = t.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND (
              t.latest_snapshot_block IS DISTINCT FROM latest.block_number
              OR t.latest_observed_block IS DISTINCT FROM latest.observed_block_number
              OR t.latest_valuation_block IS DISTINCT FROM latest.valuation_block_number
              OR abs(coalesce(nullif(t.current_value_eth, '')::numeric, 0) - coalesce(nullif(latest.current_value_eth, '')::numeric, 0)) > 0.000000000000001
              OR abs(coalesce(nullif(t.realized_pnl_eth, '')::numeric, 0) - coalesce(nullif(latest.realized_pnl_eth, '')::numeric, 0)) > 0.000000000000001
              OR abs(coalesce(nullif(t.unrealized_pnl_eth, '')::numeric, 0) - coalesce(nullif(latest.unrealized_pnl_eth, '')::numeric, 0)) > 0.000000000000001
              OR abs(coalesce(nullif(t.total_pnl_eth, '')::numeric, 0) - coalesce(nullif(latest.total_pnl_eth, '')::numeric, 0)) > 0.000000000000001
              OR abs(coalesce(nullif(t.roi, '')::numeric, 0) - coalesce(nullif(latest.roi, '')::numeric, 0)) > 0.000000000000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn zero_value_snapshot_pool_metrics_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "zero_value_snapshots_do_not_reuse_stale_pool_metrics",
        Verdict::Fail,
        "zero-value exposure snapshots do not carry stale positive pool metrics",
        "zero-value exposure snapshots with stale positive pool metrics",
        r#"
        WITH scoped AS (
            SELECT ts.id,
                   ts.trade_id,
                   ts.run_id,
                   t.token_address,
                   t.pool_address,
                   ts.state,
                   ts.block_number,
                   ts.observed_block_number,
                   ts.valuation_block_number,
                   NULLIF(ts.current_value_eth, '')::numeric AS current_value_eth,
                   NULLIF(ts.pool_liquidity_denom, '')::numeric AS pool_liquidity_denom,
                   NULLIF(ts.pool_price_to_initial_price_ratio, '')::numeric AS pool_price_to_initial_price_ratio,
                   NULLIF(ts.pool_price_denom_per_token, '')::numeric AS pool_price_denom_per_token
            FROM alpha_trading.trade_snapshots ts
            JOIN alpha_trading.trades t ON t.trade_id = ts.trade_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
        ),
        annotated AS (
            SELECT scoped.*,
                   max(
                       CASE
                           WHEN abs(coalesce(current_value_eth, 0)) <= 0.000000000000001
                                AND (
                                    (pool_liquidity_denom IS NOT NULL AND pool_liquidity_denom <= 0.001)
                                    OR (
                                        pool_price_to_initial_price_ratio IS NOT NULL
                                        AND pool_price_to_initial_price_ratio <= 0.000000001
                                    )
                                    OR (
                                        pool_price_denom_per_token IS NOT NULL
                                        AND pool_price_denom_per_token <= 0.000000000000000001
                                    )
                                )
                               THEN block_number
                       END
                   ) OVER (
                       PARTITION BY trade_id
                       ORDER BY block_number NULLS LAST, id
                       ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING
                   ) AS prior_drained_snapshot_block,
                   EXISTS (
                       SELECT 1
                       FROM alpha_trading.risk_events re
                       WHERE re.run_id = scoped.run_id
                         AND re.kind = 'liquidity_removal'
                         AND lower(re.pool_address) = lower(scoped.pool_address)
                         AND re.observed_block IS NOT NULL
                         AND re.observed_block <= COALESCE(
                             scoped.observed_block_number,
                             scoped.valuation_block_number,
                             scoped.block_number
                         )
                   ) AS has_seen_liquidity_removal
            FROM scoped
        )
        SELECT count(*)
        FROM annotated
        WHERE abs(coalesce(current_value_eth, 0)) <= 0.000000000000001
          AND state IN (
              'buy_confirmed',
              'sell_intent_created',
              'sell_submitted',
              'sell_failed',
              'sell_cancelled'
          )
          AND (
              (pool_liquidity_denom IS NOT NULL AND pool_liquidity_denom > 0.001)
              OR (
                  pool_price_to_initial_price_ratio IS NOT NULL
                  AND pool_price_to_initial_price_ratio > 0.000000001
              )
              OR (
                  pool_price_denom_per_token IS NOT NULL
                  AND pool_price_denom_per_token > 0.000000000000000001
              )
          )
          AND (
              (
                  observed_block_number IS NOT NULL
                  AND COALESCE(valuation_block_number, block_number) IS NOT NULL
                  AND observed_block_number < COALESCE(valuation_block_number, block_number)
              )
              OR prior_drained_snapshot_block IS NOT NULL
              OR has_seen_liquidity_removal
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn closed_trade_final_snapshot_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "closed_trade_final_snapshot",
        Verdict::Warn,
        "closed trades have a sell_confirmed snapshot at the exit block",
        "closed trades missing sell_confirmed exit snapshot",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades t
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND t.state = 'sell_confirmed'
          AND NOT EXISTS (
              SELECT 1
              FROM alpha_trading.trade_snapshots ts
              WHERE ts.trade_id = t.trade_id
                AND ts.block_number = t.exit_block
                AND ts.state = 'sell_confirmed'
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn closed_trade_latest_snapshot_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "closed_trade_latest_snapshot_is_terminal",
        Verdict::Fail,
        "closed trades have a terminal sell_confirmed latest snapshot",
        "closed trades whose latest snapshot is not the sell_confirmed exit snapshot",
        r#"
        WITH latest AS (
            SELECT DISTINCT ON (trade_id)
                   trade_id,
                   state,
                   block_number,
                   valuation_block_number
            FROM alpha_trading.trade_snapshots
            ORDER BY trade_id, block_number DESC NULLS LAST, id DESC
        )
        SELECT count(*)
        FROM alpha_trading.trades t
        LEFT JOIN latest ON latest.trade_id = t.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND t.state = 'sell_confirmed'
          AND (
              latest.trade_id IS NULL
              OR latest.state <> 'sell_confirmed'
              OR latest.block_number IS DISTINCT FROM t.exit_block
              OR coalesce(latest.valuation_block_number, latest.block_number) IS DISTINCT FROM t.exit_block
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}
