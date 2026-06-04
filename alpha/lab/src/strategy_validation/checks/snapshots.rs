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

pub(super) async fn no_snapshots_before_buy_confirmation_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "no_snapshots_before_buy_confirmed",
        Verdict::Fail,
        "trade snapshots are not valued before the buy_confirmed entry block",
        "snapshots whose valuation block is before the trade entry block",
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_snapshots ts
        JOIN alpha_trading.trades t
          ON t.run_id = ts.run_id
         AND t.trade_id = ts.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND t.entry_block IS NOT NULL
          AND COALESCE(ts.valuation_block_number, ts.block_number) < t.entry_block
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
              OR abs(coalesce(nullif(t.current_value_eth, '')::numeric, 0) - coalesce(nullif(latest.current_value_eth, '')::numeric, 0)) > 0.000001
              OR abs(coalesce(nullif(t.realized_pnl_eth, '')::numeric, 0) - coalesce(nullif(latest.realized_pnl_eth, '')::numeric, 0)) > 0.000001
              OR abs(coalesce(nullif(t.unrealized_pnl_eth, '')::numeric, 0) - coalesce(nullif(latest.unrealized_pnl_eth, '')::numeric, 0)) > 0.000001
              OR abs(coalesce(nullif(t.total_pnl_eth, '')::numeric, 0) - coalesce(nullif(latest.total_pnl_eth, '')::numeric, 0)) > 0.000001
              OR abs(coalesce(nullif(t.roi, '')::numeric, 0) - coalesce(nullif(latest.roi, '')::numeric, 0)) > 0.000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn observed_block_not_after_valuation_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "snapshot_observed_not_after_valuation",
        Verdict::Fail,
        "snapshot observed blocks are not after their valuation blocks",
        "snapshots whose pool observation block is after valuation block",
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_snapshots ts
        JOIN alpha_trading.trades t ON t.trade_id = ts.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND ts.observed_block_number IS NOT NULL
          AND ts.observed_block_number > COALESCE(ts.valuation_block_number, ts.block_number)
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn duplicate_snapshot_coordinate_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "no_duplicate_snapshot_coordinates",
        Verdict::Fail,
        "each trade has at most one snapshot for the same block, state, and valuation block",
        "duplicate trade snapshot coordinates",
        r#"
        WITH duplicates AS (
            SELECT ts.trade_id,
                   ts.block_number,
                   ts.state,
                   ts.valuation_block_number
            FROM alpha_trading.trade_snapshots ts
            JOIN alpha_trading.trades t ON t.trade_id = ts.trade_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
            GROUP BY ts.trade_id, ts.block_number, ts.state, ts.valuation_block_number
            HAVING count(*) > 1
        )
        SELECT count(*)
        FROM duplicates
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn duplicate_position_snapshot_coordinate_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "no_duplicate_position_snapshot_coordinates",
        Verdict::Fail,
        "each position has at most one persisted snapshot for the same block, state, and valuation block",
        "duplicate position snapshot coordinates",
        r#"
        WITH duplicates AS (
            SELECT ps.run_id,
                   ps.position_id,
                   ps.state,
                   ps.block_number,
                   ps.valuation_block_number
            FROM alpha_trading.position_snapshots ps
            JOIN alpha_trading.trades t
              ON t.run_id = ps.run_id
             AND t.trade_id = ps.trade_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
            GROUP BY ps.run_id, ps.position_id, ps.state, ps.block_number, ps.valuation_block_number
            HAVING count(*) > 1
        )
        SELECT count(*)
        FROM duplicates
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn trade_position_snapshot_mirror_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "trade_position_snapshots_match",
        Verdict::Fail,
        "trade snapshots match their source position snapshots",
        "trade and position snapshots with missing or mismatched mirror rows",
        r#"
        WITH scoped_trades AS (
            SELECT trade_id, run_id, strategy_name
            FROM alpha_trading.trades
            WHERE result_set_id = $1
              AND ($2::text IS NULL OR strategy_name = $2)
        ),
        trade_missing_position AS (
            SELECT ts.id
            FROM alpha_trading.trade_snapshots ts
            JOIN scoped_trades t
              ON t.trade_id = ts.trade_id
             AND t.run_id = ts.run_id
            WHERE NOT EXISTS (
                SELECT 1
                FROM alpha_trading.position_snapshots ps
                WHERE ps.run_id = ts.run_id
                  AND ps.trade_id = ts.trade_id
                  AND ps.position_id = ts.position_id
                  AND ps.state = ts.state
                  AND ps.block_number = ts.block_number
                  AND ps.valuation_block_number IS NOT DISTINCT FROM ts.valuation_block_number
                  AND ps.observed_block_number IS NOT DISTINCT FROM ts.observed_block_number
                  AND abs(coalesce(nullif(ps.current_value_eth, '')::numeric, 0) - coalesce(nullif(ts.current_value_eth, '')::numeric, 0)) <= 0.000001
                  AND abs(coalesce(nullif(ps.realized_profit_eth, '')::numeric, 0) - coalesce(nullif(ts.realized_pnl_eth, '')::numeric, 0)) <= 0.000001
                  AND abs(coalesce(nullif(ps.unrealized_profit_eth, '')::numeric, 0) - coalesce(nullif(ts.unrealized_pnl_eth, '')::numeric, 0)) <= 0.000001
                  AND abs(coalesce(nullif(ps.roi, '')::numeric, 0) - coalesce(nullif(ts.roi, '')::numeric, 0)) <= 0.000001
            )
        ),
        position_missing_trade AS (
            SELECT ps.id
            FROM alpha_trading.position_snapshots ps
            JOIN scoped_trades t
              ON t.trade_id = ps.trade_id
             AND t.run_id = ps.run_id
            WHERE NOT EXISTS (
                SELECT 1
                FROM alpha_trading.trade_snapshots ts
                WHERE ts.run_id = ps.run_id
                  AND ts.trade_id = ps.trade_id
                  AND ts.position_id = ps.position_id
                  AND ts.state = ps.state
                  AND ts.block_number = ps.block_number
                  AND ts.valuation_block_number IS NOT DISTINCT FROM ps.valuation_block_number
                  AND ts.observed_block_number IS NOT DISTINCT FROM ps.observed_block_number
                  AND abs(coalesce(nullif(ts.current_value_eth, '')::numeric, 0) - coalesce(nullif(ps.current_value_eth, '')::numeric, 0)) <= 0.000001
                  AND abs(coalesce(nullif(ts.realized_pnl_eth, '')::numeric, 0) - coalesce(nullif(ps.realized_profit_eth, '')::numeric, 0)) <= 0.000001
                  AND abs(coalesce(nullif(ts.unrealized_pnl_eth, '')::numeric, 0) - coalesce(nullif(ps.unrealized_profit_eth, '')::numeric, 0)) <= 0.000001
                  AND abs(coalesce(nullif(ts.roi, '')::numeric, 0) - coalesce(nullif(ps.roi, '')::numeric, 0)) <= 0.000001
            )
        )
        SELECT
            (SELECT count(*) FROM trade_missing_position)
            + (SELECT count(*) FROM position_missing_trade)
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
                           WHEN abs(coalesce(current_value_eth, 0)) <= 0.000001
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
                         AND re.pending_tx_hash IS NULL
                         AND COALESCE(re.payload->>'source', '') NOT IN (
                             'mempool_signal'
                         )
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
        WHERE abs(coalesce(current_value_eth, 0)) <= 0.000001
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

pub(super) async fn terminal_snapshot_no_pool_metrics_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "terminal_snapshots_have_no_pool_metrics",
        Verdict::Fail,
        "sell-confirmed terminal snapshots carry no pool price or liquidity metrics",
        "sell-confirmed terminal snapshots with pool price or liquidity metrics",
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_snapshots ts
        JOIN alpha_trading.trades t ON t.trade_id = ts.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND ts.state = 'sell_confirmed'
          AND (
              nullif(ts.pool_liquidity_denom, '') IS NOT NULL
              OR nullif(ts.pool_price_to_initial_price_ratio, '') IS NOT NULL
              OR nullif(ts.pool_price_denom_per_token, '') IS NOT NULL
              OR nullif(ts.pool_initial_price_denom_per_token, '') IS NOT NULL
              OR nullif(ts.pool_token_reserve, '') IS NOT NULL
              OR nullif(ts.pool_denom_symbol, '') IS NOT NULL
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
        Verdict::Fail,
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

/// (a) An open position whose pool had a mined liquidity-removal / holder-balance
/// drain must have a zero/near-zero valuation snapshot at or after the drain
/// block. Catches the Session-class bug where the position kept a positive mark
/// after the held inventory was gone.
pub(super) async fn drain_block_has_zero_snapshot_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "open_position_balance_drain_has_zero_snapshot",
        Verdict::Fail,
        "open positions with a mined drain have a zero/near-zero snapshot at/after the drain block",
        "open positions with a mined drain but no zero/near-zero snapshot at/after the drain block",
        r#"
        WITH drained AS (
            SELECT t.trade_id,
                   min(re.observed_block) AS drain_block
            FROM alpha_trading.trades t
            JOIN alpha_trading.risk_events re
              ON re.run_id = t.run_id
             AND lower(re.pool_address) = lower(t.pool_address)
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
              AND re.kind = 'liquidity_removal'
              AND re.pending_tx_hash IS NULL
              AND COALESCE(re.payload->>'source', '') NOT IN ('mempool_signal')
              AND re.observed_block IS NOT NULL
            GROUP BY t.trade_id
        )
        SELECT count(*)
        FROM drained d
        WHERE NOT EXISTS (
            SELECT 1
            FROM alpha_trading.trade_snapshots ts
            WHERE ts.trade_id = d.trade_id
              AND COALESCE(ts.valuation_block_number, ts.block_number) >= d.drain_block
              AND abs(COALESCE(NULLIF(ts.current_value_eth, '')::numeric, 0)) <= 0.000001
        )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

/// (b) After a mined drain for a position's pool, no open-state snapshot may keep
/// a positive value. A positive mark valued after the drain block means the
/// valuation used synthetic/entry inventory instead of the current held balance.
pub(super) async fn synthetic_balance_used_after_drain_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "no_positive_open_snapshot_after_drain",
        Verdict::Fail,
        "no open-state snapshot keeps a positive value after a mined drain",
        "open-state snapshots valued positive after a mined drain (synthetic/entry balance)",
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_snapshots ts
        JOIN alpha_trading.trades t ON t.trade_id = ts.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND ts.state IN (
              'buy_confirmed', 'sell_intent_created', 'sell_submitted',
              'sell_failed', 'sell_cancelled'
          )
          AND COALESCE(NULLIF(ts.current_value_eth, '')::numeric, 0) > 0.000001
          AND EXISTS (
              SELECT 1
              FROM alpha_trading.risk_events re
              WHERE re.run_id = t.run_id
                AND lower(re.pool_address) = lower(t.pool_address)
                AND re.kind = 'liquidity_removal'
                AND re.pending_tx_hash IS NULL
                AND COALESCE(re.payload->>'source', '') NOT IN ('mempool_signal')
                AND re.observed_block IS NOT NULL
                AND re.observed_block <= COALESCE(ts.valuation_block_number, ts.block_number)
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

/// (c) Once a trade has a zero/near-zero snapshot after a mined drain, no later
/// non-terminal snapshot may flip back to a positive value. This catches
/// re-valuation that resurrects a drained position without treating an unpriced
/// entry snapshot as a confirmed drain.
pub(super) async fn positive_value_after_zero_balance_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "snapshots",
        "no_positive_value_after_zero_balance",
        Verdict::Fail,
        "no non-terminal snapshot flips back to positive value after a zero-value snapshot",
        "non-terminal snapshots valued positive after an earlier zero-value snapshot",
        r#"
        WITH scoped AS (
            SELECT ts.trade_id,
                   t.run_id,
                   t.pool_address,
                   ts.block_number,
                   COALESCE(ts.valuation_block_number, ts.block_number) AS valuation_block_number,
                   ts.state,
                   NULLIF(ts.current_value_eth, '')::numeric AS current_value_eth
            FROM alpha_trading.trade_snapshots ts
            JOIN alpha_trading.trades t ON t.trade_id = ts.trade_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
        ),
        zero_block AS (
            SELECT trade_id, min(block_number) AS first_zero_block
            FROM scoped
            WHERE abs(COALESCE(current_value_eth, 0)) <= 0.000001
              AND state IN (
                  'buy_confirmed', 'sell_intent_created', 'sell_submitted',
                  'sell_failed', 'sell_cancelled'
              )
              AND EXISTS (
                  SELECT 1
                  FROM alpha_trading.risk_events re
                  WHERE re.run_id = scoped.run_id
                    AND lower(re.pool_address) = lower(scoped.pool_address)
                    AND re.kind = 'liquidity_removal'
                    AND re.pending_tx_hash IS NULL
                    AND COALESCE(re.payload->>'source', '') NOT IN ('mempool_signal')
                    AND re.observed_block IS NOT NULL
                    AND re.observed_block <= scoped.valuation_block_number
              )
            GROUP BY trade_id
        )
        SELECT count(*)
        FROM scoped s
        JOIN zero_block z ON z.trade_id = s.trade_id
        WHERE s.block_number > z.first_zero_block
          AND COALESCE(s.current_value_eth, 0) > 0.000001
          AND s.state IN (
              'buy_confirmed', 'sell_intent_created', 'sell_submitted',
              'sell_failed', 'sell_cancelled'
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}
