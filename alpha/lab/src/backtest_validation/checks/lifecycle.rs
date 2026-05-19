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
                   min(block_number) FILTER (WHERE event_type IN ('buy_failed', 'buy_cancelled')) AS buy_failed_block,
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
