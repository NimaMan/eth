use eyre::Result;
use serde_json::json;
use sqlx::PgPool;

use super::db::{scalar_i64, ResultSetRecord, StrategySummary};
use super::report::{CheckResult, Verdict};

const TOLERANCE_ETH: &str = "0.000000000000001";

pub async fn run_checks(
    pool: &PgPool,
    result_set: &ResultSetRecord,
    strategy: Option<&str>,
    strategy_summaries: &[StrategySummary],
) -> Result<Vec<CheckResult>> {
    let mut checks = Vec::new();
    checks.push(result_set_status_check(result_set));
    checks.push(strategy_rows_check(strategy_summaries, strategy));
    checks.push(trade_id_format_check(pool, &result_set.result_set_id, strategy).await?);
    checks.push(historical_mempool_scope_check(pool, result_set, strategy).await?);
    checks.push(
        submitted_events_have_decisions_check(pool, &result_set.result_set_id, strategy, "buy")
            .await?,
    );
    checks.push(
        submitted_events_have_decisions_check(pool, &result_set.result_set_id, strategy, "sell")
            .await?,
    );
    checks.push(
        risk_sell_decisions_have_prior_risk_events_check(pool, &result_set.result_set_id, strategy)
            .await?,
    );
    checks.push(
        entry_block_matches_buy_confirmation_check(pool, &result_set.result_set_id, strategy)
            .await?,
    );
    checks.push(
        exit_block_matches_sell_confirmation_check(pool, &result_set.result_set_id, strategy)
            .await?,
    );
    checks.push(
        no_exit_block_before_terminal_sell_check(pool, &result_set.result_set_id, strategy).await?,
    );
    checks
        .push(no_duplicate_terminal_events_check(pool, &result_set.result_set_id, strategy).await?);
    checks.push(lifecycle_order_check(pool, &result_set.result_set_id, strategy).await?);
    checks.push(pnl_sum_check(pool, &result_set.result_set_id, strategy).await?);
    checks.push(realized_sell_pnl_check(pool, &result_set.result_set_id, strategy).await?);
    checks
        .push(closed_trade_zero_unrealized_check(pool, &result_set.result_set_id, strategy).await?);
    checks.push(
        closed_trade_snapshot_zero_unrealized_check(pool, &result_set.result_set_id, strategy)
            .await?,
    );
    checks.push(latest_snapshot_block_check(pool, &result_set.result_set_id, strategy).await?);
    checks
        .push(closed_trade_final_snapshot_check(pool, &result_set.result_set_id, strategy).await?);
    checks.push(execution_replay_inputs_check(pool, &result_set.result_set_id, strategy).await?);
    checks.push(pnl_concentration_check(pool, &result_set.result_set_id, strategy).await?);
    Ok(checks)
}

fn result_set_status_check(result_set: &ResultSetRecord) -> CheckResult {
    let expected_status = match result_set.mode.as_str() {
        "historical" => "completed",
        "live" => "running",
        _ => "",
    };
    let verdict = if expected_status.is_empty() || result_set.status == expected_status {
        Verdict::Pass
    } else {
        Verdict::Warn
    };
    let message = if expected_status.is_empty() {
        format!(
            "result set mode `{}` has no strict lab status expectation",
            result_set.mode
        )
    } else {
        format!(
            "result set status is `{}`, expected `{}` for mode `{}`",
            result_set.status, expected_status, result_set.mode
        )
    };
    check(
        "metadata",
        "result_set_status",
        verdict,
        message,
        json!({
            "mode": result_set.mode,
            "status": result_set.status,
            "expected_status": expected_status,
        }),
    )
}

fn strategy_rows_check(
    strategy_summaries: &[StrategySummary],
    strategy: Option<&str>,
) -> CheckResult {
    let count = strategy_summaries.len();
    let trade_count: i64 = strategy_summaries
        .iter()
        .map(|summary| summary.trades)
        .sum();
    let verdict = if trade_count > 0 {
        Verdict::Pass
    } else {
        Verdict::Fail
    };
    check(
        "metadata",
        "strategy_rows",
        verdict,
        format!("{trade_count} trades found across {count} strategy rows"),
        json!({
            "strategy_filter": strategy,
            "strategy_rows": count,
            "trades": trade_count,
        }),
    )
}

async fn trade_id_format_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "identity",
        "public_trade_id_format",
        Verdict::Fail,
        "all public trade IDs use the `trd_` prefix",
        "trade IDs without `trd_` prefix",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades
        WHERE result_set_id = $1
          AND ($2::text IS NULL OR strategy_name = $2)
          AND trade_id NOT LIKE 'trd\_%' ESCAPE '\'
        "#,
        result_set_id,
        strategy,
    )
    .await
}

async fn historical_mempool_scope_check(
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

async fn submitted_events_have_decisions_check(
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

async fn risk_sell_decisions_have_prior_risk_events_check(
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
          AND sd.event_source = 'risk'
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

async fn entry_block_matches_buy_confirmation_check(
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

async fn exit_block_matches_sell_confirmation_check(
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

async fn no_exit_block_before_terminal_sell_check(
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

async fn no_duplicate_terminal_events_check(
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

async fn lifecycle_order_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "lifecycle",
        "event_block_order",
        Verdict::Fail,
        "trade events preserve buy submit <= buy fill <= sell submit <= sell fill ordering",
        "trades with impossible event block ordering",
        r#"
        WITH ev AS (
            SELECT trade_id,
                   min(block_number) FILTER (WHERE event_type = 'buy_submitted') AS buy_submitted_block,
                   min(block_number) FILTER (WHERE event_type = 'buy_confirmed') AS buy_confirmed_block,
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
              OR ev.buy_confirmed_block IS NULL
              OR ev.buy_confirmed_block < ev.buy_submitted_block
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

async fn pnl_sum_check(
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
          ) > 0.000000000000001
        "#,
        result_set_id,
        strategy,
    )
    .await
}

async fn realized_sell_pnl_check(
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
              ) > 0.000000000000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

async fn closed_trade_zero_unrealized_check(
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
              abs(coalesce(nullif(current_value_eth, '')::numeric, 0)) > 0.000000000000001
              OR abs(coalesce(nullif(unrealized_pnl_eth, '')::numeric, 0)) > 0.000000000000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

async fn closed_trade_snapshot_zero_unrealized_check(
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
              abs(coalesce(nullif(ts.current_value_eth, '')::numeric, 0)) > 0.000000000000001
              OR abs(coalesce(nullif(ts.unrealized_pnl_eth, '')::numeric, 0)) > 0.000000000000001
              OR abs(
                  coalesce(nullif(ts.total_pnl_eth, '')::numeric, 0)
                  - coalesce(nullif(ts.realized_pnl_eth, '')::numeric, 0)
              ) > 0.000000000000001
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

async fn latest_snapshot_block_check(
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

async fn closed_trade_final_snapshot_check(
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

async fn execution_replay_inputs_check(
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

async fn pnl_concentration_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    let row_count = scalar_i64(
        pool,
        r#"
        WITH ranked AS (
            SELECT coalesce(nullif(total_pnl_eth, '')::numeric, 0) AS pnl,
                   row_number() OVER (ORDER BY coalesce(nullif(total_pnl_eth, '')::numeric, 0) DESC) AS rn
            FROM alpha_trading.trades
            WHERE result_set_id = $1
              AND ($2::text IS NULL OR strategy_name = $2)
        ),
        totals AS (
            SELECT coalesce(sum(pnl), 0) AS total_pnl,
                   coalesce(sum(pnl) FILTER (WHERE rn <= 5), 0) AS top5_pnl
            FROM ranked
        )
        SELECT CASE
            WHEN abs(total_pnl) > 0.000000000000001 AND abs(top5_pnl / total_pnl) > 1.0 THEN 1
            ELSE 0
        END
        FROM totals
        "#,
        result_set_id,
        strategy,
    )
    .await?;
    let verdict = if row_count == 0 {
        Verdict::Pass
    } else {
        Verdict::Warn
    };
    Ok(check(
        "distribution",
        "top5_pnl_concentration",
        verdict,
        if row_count == 0 {
            "top five winners do not exceed 100% of aggregate PnL".to_string()
        } else {
            "top five winners exceed 100% of aggregate PnL; strategy conclusion is concentration-sensitive".to_string()
        },
        json!({
            "threshold": "abs(top5_pnl / total_pnl) <= 1.0",
            "violations": row_count,
        }),
    ))
}

async fn count_check(
    pool: &PgPool,
    category: impl Into<String>,
    code: impl Into<String>,
    nonzero_verdict: Verdict,
    pass_message: impl Into<String>,
    nonzero_message: impl Into<String>,
    sql: &str,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    let count = scalar_i64(pool, sql, result_set_id, strategy).await?;
    let verdict = if count == 0 {
        Verdict::Pass
    } else {
        nonzero_verdict
    };
    let message = if count == 0 {
        pass_message.into()
    } else {
        format!("{}: {count}", nonzero_message.into())
    };
    Ok(check(
        category,
        code,
        verdict,
        message,
        json!({
            "violations": count,
            "tolerance_eth": TOLERANCE_ETH,
        }),
    ))
}

fn check(
    category: impl Into<String>,
    code: impl Into<String>,
    verdict: Verdict,
    message: impl Into<String>,
    evidence: serde_json::Value,
) -> CheckResult {
    let code = code.into();
    let (question, description) = check_copy(&code);
    CheckResult {
        category: category.into(),
        code,
        question: question.to_string(),
        description: description.to_string(),
        verdict,
        message: message.into(),
        evidence,
    }
}

fn check_copy(code: &str) -> (&'static str, &'static str) {
    match code {
        "result_set_status" => (
            "Is this result set in the expected state?",
            "Compares result-set mode with status so completed historical results and running live results are not mixed with partial runs.",
        ),
        "strategy_rows" => (
            "Did this strategy produce trades in this result set?",
            "Counts scoped trade rows before any PnL or validation verdict is treated as meaningful.",
        ),
        "public_trade_id_format" => (
            "Are public trade identifiers opaque and stable?",
            "Requires every scoped trade_id to use the public `trd_` prefix instead of leaking legacy position IDs.",
        ),
        "historical_mempool_rows" => (
            "Did a historical backtest avoid pending mempool evidence?",
            "Checks joined risk events for pending transaction hashes; pure historical results must use mined local evidence only.",
        ),
        "buy_submitted_has_decision" => (
            "Can every buy submission be explained by a strategy decision?",
            "Joins buy_submitted trade events to same-block submit_buy strategy decisions for the same run, strategy, token, and pool.",
        ),
        "sell_submitted_has_decision" => (
            "Can every sell submission be explained by a strategy decision?",
            "Joins sell_submitted trade events to same-block submit_sell strategy decisions for the same run, strategy, token, and pool.",
        ),
        "risk_sell_has_available_signal" => (
            "Was each risk-triggered sell based on already-available evidence?",
            "Requires risk sell decisions to have a local risk event for the token/pool at or before the decision block.",
        ),
        "entry_block_matches_buy_confirmed" => (
            "Does entry_block mean the buy-confirmed block?",
            "Compares each trade entry_block with its buy_confirmed lifecycle event block.",
        ),
        "exit_block_matches_sell_confirmed" => (
            "Does exit_block mean the sell-confirmed block?",
            "Compares each closed trade exit_block with its sell_confirmed lifecycle event block.",
        ),
        "no_exit_block_before_sell_confirmed" => (
            "Do open or failed trades avoid fake exit blocks?",
            "Fails if any non-sell_confirmed trade has exit_block populated from a snapshot or pending sell.",
        ),
        "single_terminal_event_per_trade" => (
            "Does each trade have only one terminal buy and sell confirmation?",
            "Counts buy_confirmed and sell_confirmed events per trade and rejects duplicated terminal lifecycle events.",
        ),
        "event_block_order" => (
            "Are lifecycle event blocks ordered correctly?",
            "Validates buy submit <= buy confirmation <= sell submit <= sell confirmation where those events exist.",
        ),
        "total_pnl_equals_realized_plus_unrealized" => (
            "Does total PnL reconcile with realized and unrealized PnL?",
            "Checks total_pnl_eth equals realized_pnl_eth plus unrealized_pnl_eth within a small ETH tolerance.",
        ),
        "realized_sell_pnl_formula" => (
            "Does realized PnL reconcile with entry, exit, and gas?",
            "For closed trades, checks realized_pnl_eth equals exit_value_eth minus entry_cost_eth minus gas_cost_eth.",
        ),
        "closed_trade_has_no_unrealized_value" => (
            "Are closed trades fully realized?",
            "Fails if a sell-confirmed trade still carries current_value_eth or unrealized_pnl_eth.",
        ),
        "closed_trade_snapshots_have_no_unrealized_value" => (
            "Do closed-trade snapshots stay fully realized?",
            "Checks every sell_confirmed snapshot, not only the final trade row, so stale post-exit snapshots cannot corrupt aggregate PnL later.",
        ),
        "latest_snapshot_block_matches_snapshots" => (
            "Does the trade latest snapshot pointer match persisted snapshots?",
            "Compares trades.latest_snapshot_block with the max block_number in trade_snapshots for each trade.",
        ),
        "closed_trade_final_snapshot" => (
            "Does each closed trade have a final closed snapshot?",
            "Requires a sell_confirmed trade snapshot at the exit_block so the UI can show terminal valuation cleanly.",
        ),
        "closed_trade_replay_inputs_present" => (
            "Can this closed trade be independently replayed?",
            "Requires buy token amount, sell order amount, and sell-confirmed filled amount to be persisted.",
        ),
        "top5_pnl_concentration" => (
            "Is strategy PnL too concentrated in the top winners?",
            "Warns when the top five winners exceed 100% of aggregate PnL, making strategy conclusions fragile.",
        ),
        _ => (
            "What invariant is this check validating?",
            "Runs a scoped validation query against alpha_trading and reports any violating rows.",
        ),
    }
}
