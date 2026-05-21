use eyre::Result;
use sqlx::PgPool;

use super::super::report::{CheckResult, Verdict};
use super::common::count_check;

pub(super) async fn entry_cost_matches_buy_fill_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "accounting",
        "entry_cost_matches_buy_fill",
        Verdict::Fail,
        "entry_cost_eth equals the buy_confirmed filled ETH amount from the EVM simulation report",
        "trades whose entry cost differs from buy_confirmed fill",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades t
        JOIN alpha_trading.trade_events te
          ON te.trade_id = t.trade_id
         AND te.event_type = 'buy_confirmed'
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND (
              nullif(t.entry_cost_eth, '') IS NULL
              OR nullif(te.filled_amount_raw, '') IS NULL
              OR abs(
                  nullif(t.entry_cost_eth, '')::numeric
                  - (
                      nullif(te.filled_amount_raw, '')::numeric
                      / power(10::numeric, COALESCE(te.filled_amount_decimals, 18))
                    )
              ) > 0.000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn exit_value_matches_sell_fill_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "accounting",
        "exit_value_matches_sell_fill",
        Verdict::Fail,
        "exit_value_eth equals the sell_confirmed filled ETH amount from the EVM simulation report",
        "closed trades whose exit value differs from sell_confirmed fill",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades t
        JOIN alpha_trading.trade_events te
          ON te.trade_id = t.trade_id
         AND te.event_type = 'sell_confirmed'
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND (
              nullif(t.exit_value_eth, '') IS NULL
              OR nullif(te.filled_amount_raw, '') IS NULL
              OR abs(
                  nullif(t.exit_value_eth, '')::numeric
                  - (
                      nullif(te.filled_amount_raw, '')::numeric
                      / power(10::numeric, COALESCE(te.filled_amount_decimals, 18))
                    )
              ) > 0.000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn gas_cost_matches_events_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "accounting",
        "gas_cost_matches_trade_events",
        Verdict::Fail,
        "trade gas_cost_eth equals summed persisted trade event gas costs",
        "trades whose gas_cost_eth differs from trade event gas total",
        r#"
        WITH event_gas AS (
            SELECT trade_id,
                   sum(coalesce(nullif(gas_cost_eth, '')::numeric, 0)) AS gas_cost_eth
            FROM alpha_trading.trade_events
            GROUP BY trade_id
        )
        SELECT count(*)
        FROM alpha_trading.trades t
        LEFT JOIN event_gas ON event_gas.trade_id = t.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND abs(
              coalesce(nullif(t.gas_cost_eth, '')::numeric, 0)
              - coalesce(event_gas.gas_cost_eth, 0)
          ) > 0.000001
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn failed_sell_gas_has_snapshot_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "accounting",
        "failed_sell_gas_has_same_block_snapshot",
        Verdict::Fail,
        "failed sell gas is reflected by a same-block sell_failed snapshot",
        "sell_failed gas events without same-block sell_failed snapshot",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades t
        JOIN alpha_trading.trade_events te
          ON te.trade_id = t.trade_id
         AND te.event_type = 'sell_failed'
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND coalesce(nullif(te.gas_cost_eth, '')::numeric, 0) > 0
          AND NOT EXISTS (
              SELECT 1
              FROM alpha_trading.trade_snapshots ts
              WHERE ts.trade_id = te.trade_id
                AND ts.state = 'sell_failed'
                AND ts.block_number = te.block_number
                AND COALESCE(ts.valuation_block_number, ts.block_number) = te.block_number
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn pnl_sum_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "accounting",
        "total_pnl_equals_realized_plus_unrealized",
        Verdict::Fail,
        "total PnL equals realized plus unrealized PnL within tolerance",
        "trades with inconsistent total PnL",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades
        WHERE result_set_id = $1
          AND ($2::text IS NULL OR strategy_name = $2)
          AND abs(
              coalesce(nullif(total_pnl_eth, '')::numeric, 0)
              - coalesce(nullif(realized_pnl_eth, '')::numeric, 0)
              - coalesce(nullif(unrealized_pnl_eth, '')::numeric, 0)
          ) > 0.000001
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn open_trade_pnl_formula_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "accounting",
        "open_trade_pnl_formula",
        Verdict::Fail,
        "open trade PnL equals realized gas loss plus current value minus entry cost",
        "open trades with inconsistent realized or unrealized PnL",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades
        WHERE result_set_id = $1
          AND ($2::text IS NULL OR strategy_name = $2)
          AND state IN (
              'buy_confirmed',
              'sell_intent_created',
              'sell_submitted',
              'sell_failed',
              'sell_cancelled'
          )
          AND (
              nullif(entry_cost_eth, '') IS NULL
              OR nullif(current_value_eth, '') IS NULL
              OR nullif(realized_pnl_eth, '') IS NULL
              OR nullif(unrealized_pnl_eth, '') IS NULL
              OR nullif(gas_cost_eth, '') IS NULL
              OR abs(
                  coalesce(nullif(realized_pnl_eth, '')::numeric, 0)
                  + coalesce(nullif(gas_cost_eth, '')::numeric, 0)
              ) > 0.000001
              OR abs(
                  coalesce(nullif(unrealized_pnl_eth, '')::numeric, 0)
                  - (
                      coalesce(nullif(current_value_eth, '')::numeric, 0)
                      - coalesce(nullif(entry_cost_eth, '')::numeric, 0)
                    )
              ) > 0.000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn open_snapshot_pnl_formula_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "accounting",
        "open_snapshot_pnl_formula",
        Verdict::Fail,
        "open-state snapshots reconcile current value, entry cost, and gas through their valuation block",
        "open-state snapshots with inconsistent realized or unrealized PnL",
        r#"
        WITH event_gas AS (
            SELECT ts.id,
                   coalesce(sum(coalesce(nullif(te.gas_cost_eth, '')::numeric, 0)), 0) AS gas_cost_eth
            FROM alpha_trading.trade_snapshots ts
            LEFT JOIN alpha_trading.trade_events te
              ON te.trade_id = ts.trade_id
             AND te.block_number <= COALESCE(ts.valuation_block_number, ts.block_number)
            GROUP BY ts.id
        )
        SELECT count(*)
        FROM alpha_trading.trade_snapshots ts
        JOIN alpha_trading.trades t ON t.trade_id = ts.trade_id
        JOIN event_gas ON event_gas.id = ts.id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND ts.state IN (
              'buy_confirmed',
              'sell_intent_created',
              'sell_submitted',
              'sell_failed',
              'sell_cancelled'
          )
          AND (
              nullif(t.entry_cost_eth, '') IS NULL
              OR nullif(ts.current_value_eth, '') IS NULL
              OR nullif(ts.realized_pnl_eth, '') IS NULL
              OR nullif(ts.unrealized_pnl_eth, '') IS NULL
              OR abs(
                  coalesce(nullif(ts.realized_pnl_eth, '')::numeric, 0)
                  + coalesce(event_gas.gas_cost_eth, 0)
              ) > 0.000001
              OR abs(
                  coalesce(nullif(ts.unrealized_pnl_eth, '')::numeric, 0)
                  - (
                      coalesce(nullif(ts.current_value_eth, '')::numeric, 0)
                      - coalesce(nullif(t.entry_cost_eth, '')::numeric, 0)
                    )
              ) > 0.000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn realized_sell_pnl_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "accounting",
        "realized_sell_pnl_formula",
        Verdict::Fail,
        "closed trade realized PnL equals exit value minus entry cost minus gas",
        "closed trades with inconsistent realized PnL",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades
        WHERE result_set_id = $1
          AND ($2::text IS NULL OR strategy_name = $2)
          AND state = 'sell_confirmed'
          AND (
              entry_cost_eth IS NULL
              OR exit_value_eth IS NULL
              OR realized_pnl_eth IS NULL
              OR gas_cost_eth IS NULL
              OR abs(
                  coalesce(nullif(realized_pnl_eth, '')::numeric, 0)
                  - (
                      coalesce(nullif(exit_value_eth, '')::numeric, 0)
                      - coalesce(nullif(entry_cost_eth, '')::numeric, 0)
                      - coalesce(nullif(gas_cost_eth, '')::numeric, 0)
                    )
              ) > 0.000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn closed_trade_zero_unrealized_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "accounting",
        "closed_trade_has_no_unrealized_value",
        Verdict::Fail,
        "closed trades carry no current value or unrealized PnL",
        "closed trades with nonzero current value or unrealized PnL",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades
        WHERE result_set_id = $1
          AND ($2::text IS NULL OR strategy_name = $2)
          AND state = 'sell_confirmed'
          AND (
              abs(coalesce(nullif(current_value_eth, '')::numeric, 0)) > 0.000001
              OR abs(coalesce(nullif(unrealized_pnl_eth, '')::numeric, 0)) > 0.000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn closed_trade_snapshot_zero_unrealized_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "accounting",
        "closed_trade_snapshots_have_no_unrealized_value",
        Verdict::Fail,
        "sell-confirmed snapshots carry no current value or unrealized PnL",
        "sell-confirmed snapshots with nonzero current value, unrealized PnL, or inconsistent total",
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_snapshots ts
        JOIN alpha_trading.trades t ON t.trade_id = ts.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND ts.state = 'sell_confirmed'
          AND (
              abs(coalesce(nullif(ts.current_value_eth, '')::numeric, 0)) > 0.000001
              OR abs(coalesce(nullif(ts.unrealized_pnl_eth, '')::numeric, 0)) > 0.000001
              OR abs(
                  coalesce(nullif(ts.total_pnl_eth, '')::numeric, 0)
                  - coalesce(nullif(ts.realized_pnl_eth, '')::numeric, 0)
              ) > 0.000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}
