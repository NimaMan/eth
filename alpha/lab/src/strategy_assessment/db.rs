use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

const TOLERANCE_ETH: f64 = 0.000000000000001;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AssessmentMetrics {
    pub trade_count: i64,
    pub closed_trades: i64,
    pub open_trades: i64,
    pub failed_trades: i64,
    pub exposure_trades: i64,
    pub failed_exit_exposure_trades: i64,
    pub winner_count: i64,
    pub loser_count: i64,
    pub total_entry_cost_eth: f64,
    pub realized_pnl_eth: f64,
    pub unrealized_pnl_eth: f64,
    pub total_pnl_eth: f64,
    pub gross_profit_eth: f64,
    pub gross_loss_eth: f64,
    pub profit_factor: Option<f64>,
    pub top1_winner_pnl_eth: f64,
    pub top5_winner_pnl_eth: f64,
    pub top10_winner_pnl_eth: f64,
    pub pnl_ex_top1_eth: f64,
    pub pnl_ex_top5_eth: f64,
    pub pnl_ex_top10_eth: f64,
    pub top1_share_of_net_pnl_percent: Option<f64>,
    pub top5_share_of_net_pnl_percent: Option<f64>,
    pub top10_share_of_net_pnl_percent: Option<f64>,
    pub top5_loser_loss_eth: f64,
    pub top5_loser_share_of_gross_loss_percent: Option<f64>,
    pub exposure_entry_cost_eth: f64,
    pub exposure_current_value_eth: f64,
    pub exposure_unrealized_pnl_eth: f64,
    pub exposure_drawdown_eth: f64,
    pub exposure_to_capital_percent: Option<f64>,
    pub exposure_unrealized_to_net_pnl_percent: Option<f64>,
}

pub async fn load_assessment_metrics(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<AssessmentMetrics> {
    let row = sqlx::query(
        r#"
        WITH scoped AS (
            SELECT trade_id,
                   lower(state) AS state,
                   coalesce(nullif(entry_cost_eth, '')::numeric, 0) AS entry_cost_eth,
                   coalesce(nullif(current_value_eth, '')::numeric, 0) AS current_value_eth,
                   coalesce(nullif(realized_pnl_eth, '')::numeric, 0) AS realized_pnl_eth,
                   coalesce(nullif(unrealized_pnl_eth, '')::numeric, 0) AS unrealized_pnl_eth,
                   coalesce(nullif(total_pnl_eth, '')::numeric, 0) AS total_pnl_eth
            FROM alpha_trading.trades
            WHERE result_set_id = $1
              AND ($2::text IS NULL OR strategy_name = $2)
        ),
        ranked_winners AS (
            SELECT total_pnl_eth AS pnl,
                   row_number() OVER (ORDER BY total_pnl_eth DESC, trade_id ASC) AS rn
            FROM scoped
            WHERE total_pnl_eth > 0
        ),
        ranked_losers AS (
            SELECT abs(total_pnl_eth) AS loss,
                   row_number() OVER (ORDER BY total_pnl_eth ASC, trade_id ASC) AS rn
            FROM scoped
            WHERE total_pnl_eth < 0
        ),
        totals AS (
            SELECT count(*) AS trade_count,
                   count(*) FILTER (WHERE state = 'sell_confirmed') AS closed_trades,
                   count(*) FILTER (
                       WHERE state NOT IN ('sell_confirmed', 'buy_failed', 'buy_cancelled', 'cancelled', 'scammed', 'failed')
                   ) AS open_trades,
                   count(*) FILTER (WHERE state IN ('buy_failed', 'sell_failed', 'failed', 'cancelled')) AS failed_trades,
                   count(*) FILTER (
                       WHERE state IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                   ) AS exposure_trades,
                   count(*) FILTER (WHERE state IN ('sell_failed', 'sell_cancelled')) AS failed_exit_exposure_trades,
                   count(*) FILTER (WHERE total_pnl_eth > 0) AS winner_count,
                   count(*) FILTER (WHERE total_pnl_eth < 0) AS loser_count,
                   coalesce(sum(entry_cost_eth), 0) AS total_entry_cost_eth,
                   coalesce(sum(realized_pnl_eth), 0) AS realized_pnl_eth,
                   coalesce(sum(unrealized_pnl_eth), 0) AS unrealized_pnl_eth,
                   coalesce(sum(total_pnl_eth), 0) AS total_pnl_eth,
                   coalesce(sum(CASE WHEN total_pnl_eth > 0 THEN total_pnl_eth ELSE 0 END), 0) AS gross_profit_eth,
                   abs(coalesce(sum(CASE WHEN total_pnl_eth < 0 THEN total_pnl_eth ELSE 0 END), 0)) AS gross_loss_eth,
                   coalesce(sum(
                       CASE
                           WHEN state IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN entry_cost_eth
                           ELSE 0
                       END
                   ), 0) AS exposure_entry_cost_eth,
                   coalesce(sum(
                       CASE
                           WHEN state IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN current_value_eth
                           ELSE 0
                       END
                   ), 0) AS exposure_current_value_eth,
                   coalesce(sum(
                       CASE
                           WHEN state IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN unrealized_pnl_eth
                           ELSE 0
                       END
                   ), 0) AS exposure_unrealized_pnl_eth,
                   coalesce(sum(
                       CASE
                           WHEN state IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                           THEN greatest(entry_cost_eth - current_value_eth, 0)
                           ELSE 0
                       END
                   ), 0) AS exposure_drawdown_eth
            FROM scoped
        ),
        winner_totals AS (
            SELECT coalesce(sum(pnl) FILTER (WHERE rn <= 1), 0) AS top1_winner_pnl_eth,
                   coalesce(sum(pnl) FILTER (WHERE rn <= 5), 0) AS top5_winner_pnl_eth,
                   coalesce(sum(pnl) FILTER (WHERE rn <= 10), 0) AS top10_winner_pnl_eth
            FROM ranked_winners
        ),
        loser_totals AS (
            SELECT coalesce(sum(loss) FILTER (WHERE rn <= 5), 0) AS top5_loser_loss_eth
            FROM ranked_losers
        )
        SELECT trade_count::bigint,
               closed_trades::bigint,
               open_trades::bigint,
               failed_trades::bigint,
               exposure_trades::bigint,
               failed_exit_exposure_trades::bigint,
               winner_count::bigint,
               loser_count::bigint,
               total_entry_cost_eth::float8,
               realized_pnl_eth::float8,
               unrealized_pnl_eth::float8,
               total_pnl_eth::float8,
               gross_profit_eth::float8,
               gross_loss_eth::float8,
               CASE
                   WHEN gross_loss_eth > $3::numeric THEN (gross_profit_eth / gross_loss_eth)::float8
                   ELSE NULL
               END AS profit_factor,
               top1_winner_pnl_eth::float8,
               top5_winner_pnl_eth::float8,
               top10_winner_pnl_eth::float8,
               (total_pnl_eth - top1_winner_pnl_eth)::float8 AS pnl_ex_top1_eth,
               (total_pnl_eth - top5_winner_pnl_eth)::float8 AS pnl_ex_top5_eth,
               (total_pnl_eth - top10_winner_pnl_eth)::float8 AS pnl_ex_top10_eth,
               CASE
                   WHEN abs(total_pnl_eth) > $3::numeric THEN (top1_winner_pnl_eth / total_pnl_eth * 100)::float8
                   ELSE NULL
               END AS top1_share_of_net_pnl_percent,
               CASE
                   WHEN abs(total_pnl_eth) > $3::numeric THEN (top5_winner_pnl_eth / total_pnl_eth * 100)::float8
                   ELSE NULL
               END AS top5_share_of_net_pnl_percent,
               CASE
                   WHEN abs(total_pnl_eth) > $3::numeric THEN (top10_winner_pnl_eth / total_pnl_eth * 100)::float8
                   ELSE NULL
               END AS top10_share_of_net_pnl_percent,
               top5_loser_loss_eth::float8,
               CASE
                   WHEN gross_loss_eth > $3::numeric THEN (top5_loser_loss_eth / gross_loss_eth * 100)::float8
                   ELSE NULL
               END AS top5_loser_share_of_gross_loss_percent,
               exposure_entry_cost_eth::float8,
               exposure_current_value_eth::float8,
               exposure_unrealized_pnl_eth::float8,
               exposure_drawdown_eth::float8,
               CASE
                   WHEN total_entry_cost_eth > $3::numeric THEN (exposure_entry_cost_eth / total_entry_cost_eth * 100)::float8
                   ELSE NULL
               END AS exposure_to_capital_percent,
               CASE
                   WHEN abs(total_pnl_eth) > $3::numeric THEN (exposure_unrealized_pnl_eth / total_pnl_eth * 100)::float8
                   ELSE NULL
               END AS exposure_unrealized_to_net_pnl_percent
        FROM totals
        CROSS JOIN winner_totals
        CROSS JOIN loser_totals
        "#,
    )
    .bind(result_set_id)
    .bind(strategy)
    .bind(TOLERANCE_ETH)
    .fetch_one(pool)
    .await
    .wrap_err("failed to load strategy assessment metrics")?;

    Ok(AssessmentMetrics {
        trade_count: int(&row, "trade_count")?,
        closed_trades: int(&row, "closed_trades")?,
        open_trades: int(&row, "open_trades")?,
        failed_trades: int(&row, "failed_trades")?,
        exposure_trades: int(&row, "exposure_trades")?,
        failed_exit_exposure_trades: int(&row, "failed_exit_exposure_trades")?,
        winner_count: int(&row, "winner_count")?,
        loser_count: int(&row, "loser_count")?,
        total_entry_cost_eth: float(&row, "total_entry_cost_eth")?,
        realized_pnl_eth: float(&row, "realized_pnl_eth")?,
        unrealized_pnl_eth: float(&row, "unrealized_pnl_eth")?,
        total_pnl_eth: float(&row, "total_pnl_eth")?,
        gross_profit_eth: float(&row, "gross_profit_eth")?,
        gross_loss_eth: float(&row, "gross_loss_eth")?,
        profit_factor: optional_float(&row, "profit_factor")?,
        top1_winner_pnl_eth: float(&row, "top1_winner_pnl_eth")?,
        top5_winner_pnl_eth: float(&row, "top5_winner_pnl_eth")?,
        top10_winner_pnl_eth: float(&row, "top10_winner_pnl_eth")?,
        pnl_ex_top1_eth: float(&row, "pnl_ex_top1_eth")?,
        pnl_ex_top5_eth: float(&row, "pnl_ex_top5_eth")?,
        pnl_ex_top10_eth: float(&row, "pnl_ex_top10_eth")?,
        top1_share_of_net_pnl_percent: optional_float(&row, "top1_share_of_net_pnl_percent")?,
        top5_share_of_net_pnl_percent: optional_float(&row, "top5_share_of_net_pnl_percent")?,
        top10_share_of_net_pnl_percent: optional_float(&row, "top10_share_of_net_pnl_percent")?,
        top5_loser_loss_eth: float(&row, "top5_loser_loss_eth")?,
        top5_loser_share_of_gross_loss_percent: optional_float(
            &row,
            "top5_loser_share_of_gross_loss_percent",
        )?,
        exposure_entry_cost_eth: float(&row, "exposure_entry_cost_eth")?,
        exposure_current_value_eth: float(&row, "exposure_current_value_eth")?,
        exposure_unrealized_pnl_eth: float(&row, "exposure_unrealized_pnl_eth")?,
        exposure_drawdown_eth: float(&row, "exposure_drawdown_eth")?,
        exposure_to_capital_percent: optional_float(&row, "exposure_to_capital_percent")?,
        exposure_unrealized_to_net_pnl_percent: optional_float(
            &row,
            "exposure_unrealized_to_net_pnl_percent",
        )?,
    })
}

fn int(row: &sqlx::postgres::PgRow, name: &str) -> Result<i64> {
    row.try_get(name)
        .wrap_err_with(|| format!("failed to decode metric `{name}`"))
}

fn float(row: &sqlx::postgres::PgRow, name: &str) -> Result<f64> {
    row.try_get(name)
        .wrap_err_with(|| format!("failed to decode metric `{name}`"))
}

fn optional_float(row: &sqlx::postgres::PgRow, name: &str) -> Result<Option<f64>> {
    row.try_get(name)
        .wrap_err_with(|| format!("failed to decode metric `{name}`"))
}
