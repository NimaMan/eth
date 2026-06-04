use eyre::Result;
use sqlx::PgPool;

use super::super::report::{CheckResult, Verdict};
use super::common::count_check;

pub(super) async fn entry_block_matches_buy_confirmation_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "lifecycle",
        "entry_block_matches_buy_confirmed",
        Verdict::Fail,
        "trade entry_block matches the buy_confirmed event block",
        "trades whose entry_block differs from buy_confirmed block",
        r#"
        WITH ev AS (
            SELECT trade_id,
                   min(block_number) FILTER (WHERE event_type = 'buy_confirmed') AS buy_confirmed_block
            FROM alpha_trading.trade_events
            GROUP BY trade_id
        )
        SELECT count(*)
        FROM alpha_trading.trades t
        LEFT JOIN ev ON ev.trade_id = t.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND (t.entry_block IS DISTINCT FROM ev.buy_confirmed_block)
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn exit_block_matches_sell_confirmation_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "lifecycle",
        "exit_block_matches_sell_confirmed",
        Verdict::Fail,
        "sell_confirmed trades have exit_block equal to the sell_confirmed event block",
        "sell_confirmed trades with mismatched exit_block",
        r#"
        WITH ev AS (
            SELECT trade_id,
                   min(block_number) FILTER (WHERE event_type = 'sell_confirmed') AS sell_confirmed_block
            FROM alpha_trading.trade_events
            GROUP BY trade_id
        )
        SELECT count(*)
        FROM alpha_trading.trades t
        LEFT JOIN ev ON ev.trade_id = t.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND t.state = 'sell_confirmed'
          AND (t.exit_block IS DISTINCT FROM ev.sell_confirmed_block)
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn no_exit_block_before_terminal_sell_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "lifecycle",
        "no_exit_block_before_sell_confirmed",
        Verdict::Fail,
        "non-closed trades do not carry inferred exit_block values",
        "non-sell_confirmed trades with exit_block populated",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades
        WHERE result_set_id = $1
          AND ($2::text IS NULL OR strategy_name = $2)
          AND state <> 'sell_confirmed'
          AND exit_block IS NOT NULL
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn trade_position_rollup_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "rollups",
        "trade_rollup_matches_position",
        Verdict::Fail,
        "trade rollup rows match their source position rows for lifecycle fields",
        "trades whose state, order ids, blocks, or protocol differ from positions",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades t
        LEFT JOIN alpha_trading.positions p
          ON p.run_id = t.run_id
         AND p.position_id = t.position_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND (
              p.position_id IS NULL
              OR t.state IS DISTINCT FROM p.state
              OR t.entry_order_id IS DISTINCT FROM p.entry_order_id
              OR t.exit_order_id IS DISTINCT FROM p.exit_order_id
              OR t.entry_block IS DISTINCT FROM p.entry_block
              OR t.exit_block IS DISTINCT FROM p.exit_block
              OR t.protocol IS DISTINCT FROM p.protocol
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn execution_report_trade_event_mirror_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "lifecycle",
        "execution_reports_mirror_trade_events",
        Verdict::Fail,
        "execution report rows have matching trade event rows",
        "execution report/trade event lifecycle rows with mismatched counts",
        r#"
        WITH scoped_trades AS (
            SELECT run_id, trade_id
            FROM alpha_trading.trades
            WHERE result_set_id = $1
              AND ($2::text IS NULL OR strategy_name = $2)
        ),
        reports AS (
            SELECT er.run_id,
                   er.trade_id,
                   er.order_id,
                   er.order_side,
                   er.status,
                   er.block_number,
                   count(*) AS report_count
            FROM alpha_trading.execution_reports er
            JOIN scoped_trades scoped
              ON scoped.run_id = er.run_id
             AND scoped.trade_id = er.trade_id
            WHERE er.order_side IN ('buy', 'sell')
            GROUP BY er.run_id, er.trade_id, er.order_id, er.order_side,
                     er.status, er.block_number
        ),
        events AS (
            SELECT te.run_id,
                   te.trade_id,
                   te.order_id,
                   te.order_side,
                   te.status,
                   te.block_number,
                   count(*) AS event_count
            FROM alpha_trading.trade_events te
            JOIN scoped_trades scoped
              ON scoped.run_id = te.run_id
             AND scoped.trade_id = te.trade_id
            WHERE te.order_side IN ('buy', 'sell')
            GROUP BY te.run_id, te.trade_id, te.order_id, te.order_side,
                     te.status, te.block_number
        ),
        mismatches AS (
            SELECT COALESCE(reports.report_count, 0) AS report_count,
                   COALESCE(events.event_count, 0) AS event_count
            FROM reports
            FULL OUTER JOIN events
              ON events.run_id = reports.run_id
             AND events.trade_id = reports.trade_id
             AND events.order_id = reports.order_id
             AND events.order_side = reports.order_side
             AND events.status = reports.status
             AND events.block_number IS NOT DISTINCT FROM reports.block_number
            WHERE COALESCE(reports.report_count, 0)
               <> COALESCE(events.event_count, 0)
        )
        SELECT count(*)
        FROM mismatches
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn single_submitted_event_per_order_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "lifecycle",
        "single_submitted_event_per_order",
        Verdict::Fail,
        "each buy/sell order has exactly one submitted event and at most one terminal event",
        "orders with missing/duplicate submitted events or duplicate terminal events",
        r#"
        WITH order_events AS (
            SELECT te.run_id,
                   te.trade_id,
                   te.order_id,
                   te.order_side,
                   count(*) FILTER (WHERE te.status = 'submitted') AS submitted_count,
                   count(*) FILTER (WHERE te.status IN ('confirmed', 'failed', 'cancelled')) AS terminal_count
            FROM alpha_trading.trade_events te
            JOIN alpha_trading.trades t
              ON t.run_id = te.run_id
             AND t.trade_id = te.trade_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
              AND te.order_side IN ('buy', 'sell')
            GROUP BY te.run_id, te.trade_id, te.order_id, te.order_side
        )
        SELECT count(*)
        FROM order_events
        WHERE submitted_count <> 1
           OR terminal_count > 1
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn no_duplicate_terminal_events_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "lifecycle",
        "single_terminal_event_per_trade",
        Verdict::Fail,
        "each trade has at most one buy_confirmed and one sell_confirmed event",
        "trades with duplicate buy_confirmed or sell_confirmed events",
        r#"
        WITH ev AS (
            SELECT trade_id,
                   count(*) FILTER (WHERE event_type = 'buy_confirmed') AS buy_confirmed_count,
                   count(*) FILTER (WHERE event_type = 'sell_confirmed') AS sell_confirmed_count
            FROM alpha_trading.trade_events
            GROUP BY trade_id
        )
        SELECT count(*)
        FROM alpha_trading.trades t
        JOIN ev ON ev.trade_id = t.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND (ev.buy_confirmed_count > 1 OR ev.sell_confirmed_count > 1)
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn lifecycle_order_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "lifecycle",
        "event_block_order",
        Verdict::Fail,
        "trade events preserve buy submit <= buy terminal and confirmed-buy sell ordering",
        "trades with impossible event block ordering",
        r#"
        WITH ev AS (
            SELECT trade_id,
                   min(block_number) FILTER (WHERE event_type = 'buy_submitted') AS buy_submitted_block,
                   min(block_number) FILTER (WHERE event_type = 'buy_confirmed') AS buy_confirmed_block,
                   min(block_number) FILTER (WHERE event_type IN ('buy_deferred', 'buy_failed', 'buy_cancelled')) AS buy_failed_block,
                   min(block_number) FILTER (WHERE event_type = 'sell_submitted') AS sell_submitted_block,
                   min(block_number) FILTER (WHERE event_type = 'sell_confirmed') AS sell_confirmed_block
            FROM alpha_trading.trade_events
            GROUP BY trade_id
        )
        SELECT count(*)
        FROM alpha_trading.trades t
        JOIN ev ON ev.trade_id = t.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND (
              ev.buy_submitted_block IS NULL
              OR (ev.buy_confirmed_block IS NULL AND ev.buy_failed_block IS NULL)
              OR ev.buy_confirmed_block < ev.buy_submitted_block
              OR ev.buy_failed_block < ev.buy_submitted_block
              OR (
                  ev.buy_confirmed_block IS NULL
                  AND (ev.sell_submitted_block IS NOT NULL OR ev.sell_confirmed_block IS NOT NULL)
              )
              OR (ev.sell_submitted_block IS NOT NULL AND ev.sell_submitted_block < ev.buy_confirmed_block)
              OR (ev.sell_confirmed_block IS NOT NULL AND ev.sell_submitted_block IS NULL)
              OR (ev.sell_confirmed_block IS NOT NULL AND ev.sell_confirmed_block < ev.sell_submitted_block)
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn active_hold_limit_exit_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "lifecycle",
        "active_hold_limit_submits_exit",
        Verdict::Fail,
        "buy-confirmed trades submit an exit once persisted active-hold observations reach max_hold_blocks",
        "buy-confirmed trades over active-hold limit without a sell submission",
        r#"
        WITH strategy_cfg AS (
            SELECT spec->>'strategy_name' AS strategy_name,
                   NULLIF(spec->>'max_hold_blocks', '')::bigint AS max_hold_blocks
            FROM alpha_trading.backtest_result_sets rs
            CROSS JOIN LATERAL jsonb_array_elements(
                CASE
                    WHEN jsonb_typeof(rs.config->'strategies') = 'array'
                    THEN rs.config->'strategies'
                    ELSE '[]'::jsonb
                END
            ) AS spec
            WHERE rs.result_set_id = $1
            UNION ALL
            SELECT rs.config->>'strategy_name' AS strategy_name,
                   NULLIF(rs.config->>'max_hold_blocks', '')::bigint AS max_hold_blocks
            FROM alpha_trading.backtest_result_sets rs
            WHERE rs.result_set_id = $1
              AND rs.config ? 'strategy_name'
        ),
        scoped AS (
            SELECT t.trade_id,
                   t.run_id,
                   t.strategy_name,
                   t.token_address,
                   t.pool_address,
                   t.entry_block,
                   cfg.max_hold_blocks
            FROM alpha_trading.trades t
            JOIN strategy_cfg cfg
              ON cfg.strategy_name = t.strategy_name
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
              AND t.state = 'buy_confirmed'
              AND t.entry_block IS NOT NULL
              AND cfg.max_hold_blocks IS NOT NULL
        ),
        active AS (
            SELECT s.trade_id,
                   count(DISTINCT sd.block_number) FILTER (
                       WHERE sd.reason = 'position_open_no_exit'
                         AND sd.action = 'hold'
                         AND sd.block_number IS NOT NULL
                   ) AS active_hold_blocks
            FROM scoped s
            LEFT JOIN alpha_trading.strategy_decisions sd
              ON sd.run_id = s.run_id
             AND sd.strategy_name = s.strategy_name
             AND lower(sd.token_address) = lower(s.token_address)
             AND lower(sd.pool_address) = lower(s.pool_address)
             AND sd.block_number >= s.entry_block
            GROUP BY s.trade_id
        )
        SELECT count(*)
        FROM scoped s
        JOIN active ON active.trade_id = s.trade_id
        WHERE active.active_hold_blocks >= s.max_hold_blocks
          AND NOT EXISTS (
              SELECT 1
              FROM alpha_trading.trade_events te
              WHERE te.trade_id = s.trade_id
                AND te.event_type = 'sell_submitted'
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

/// S6 lifecycle gate (cause-agnostic value-zero close enforcement). The three
/// existing S6 checks only assert snapshot VALUATION invariants and therefore
/// pass on both a pre-fix run (positions left open after a confirmed drain) and
/// the fixed run, so they do not discriminate. This gate closes that gap: a
/// position whose pool suffered a mined, value-destroying risk event
/// (`liquidity_removal` / `scam_confirmed`, mined evidence only — pending
/// mempool signals excluded) at or before the position's latest observed block
/// MUST be terminalized at permanently-zero value. The required terminal state
/// is `terminal_zero`; the legacy `scammed` label is accepted for back-compat,
/// and a clean `sell_confirmed` exit is also acceptable. Any other state
/// (`buy_confirmed`, `sell_failed`, `sell_cancelled`, or any non-terminal/open
/// state) at run end despite the drain is a FAIL: the drain-close was missed and
/// the position is left lingering open.
pub(super) async fn drained_position_reaches_terminal_zero_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "lifecycle",
        "drained_position_reaches_terminal_zero",
        Verdict::Fail,
        "positions with a mined value-destroying drain are terminalized at zero value",
        "positions left non-terminal/open despite a mined value-destroying drain",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades t
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND t.state NOT IN ('terminal_zero', 'scammed', 'sell_confirmed')
          AND EXISTS (
              SELECT 1
              FROM alpha_trading.risk_events re
              WHERE re.run_id = t.run_id
                AND lower(re.pool_address) = lower(t.pool_address)
                AND re.kind IN ('liquidity_removal', 'scam_confirmed')
                AND re.pending_tx_hash IS NULL
                AND COALESCE(re.payload->>'source', '') NOT IN ('mempool_signal')
                AND re.observed_block IS NOT NULL
                AND re.observed_block <= COALESCE(
                    t.latest_observed_block,
                    t.latest_valuation_block,
                    t.latest_snapshot_block,
                    t.exit_block,
                    t.entry_block
                )
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}
