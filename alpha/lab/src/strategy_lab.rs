use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::{load_run_metadata, render, RunMetadata};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyReport {
    pub run: RunMetadata,
    pub summary: RunSummary,
    pub concentration: PnlConcentration,
    pub issue_flags: Vec<IssueFlag>,
    pub failures: Vec<FailureBucket>,
    pub buy_failed_entries: Vec<BuyFailedEntry>,
    pub open_failed_exits: Vec<OpenFailedExit>,
    pub protocols: Vec<ProtocolBucket>,
    pub top_winners: Vec<PositionRank>,
    pub worst_losers: Vec<PositionRank>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RunSummary {
    pub positions: i64,
    pub open_positions: i64,
    pub failed_positions: i64,
    pub buy_failed_positions: i64,
    pub sell_failed_positions: i64,
    pub entry_cost_eth: String,
    pub execution_reports: i64,
    pub confirmed_reports: i64,
    pub failed_reports: i64,
    pub snapshots: i64,
    pub snapshot_positions: i64,
    pub open_without_snapshot: i64,
    pub zero_decimal_nonzero_raw: i64,
    pub latest_current_value_eth: String,
    pub realized_pnl_eth: String,
    pub unrealized_pnl_eth: String,
    pub total_pnl_eth: String,
    pub total_roi_on_open_cost: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PnlConcentration {
    pub snapshot_positions: i64,
    pub total_pnl_eth: String,
    pub pnl_ex_top1_eth: String,
    pub pnl_ex_top2_eth: String,
    pub pnl_ex_top5_eth: String,
    pub pnl_ex_top10_eth: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IssueFlag {
    pub severity: String,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailureBucket {
    pub error_class: String,
    pub reports: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuyFailedEntry {
    pub token_address: String,
    pub pool_address: String,
    pub failed_block: Option<i64>,
    pub protocol: String,
    pub denom_symbol: String,
    pub observed_can_buy: Option<bool>,
    pub observed_can_sell: Option<bool>,
    pub error_class: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenFailedExit {
    pub token_address: String,
    pub pool_address: String,
    pub entry_block: Option<i64>,
    pub failed_reports: i64,
    pub first_failed_block: Option<i64>,
    pub last_failed_block: Option<i64>,
    pub latest_snapshot_block: Option<i64>,
    pub current_value_eth: Option<String>,
    pub pnl_eth: Option<String>,
    pub error_class: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolBucket {
    pub protocol: String,
    pub denom_symbol: String,
    pub confirmed_buys: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionRank {
    pub token_address: String,
    pub pool_address: String,
    pub entry_block: Option<i64>,
    pub entry_cost_eth: Option<String>,
    pub entry_token_amount: Option<String>,
    pub latest_snapshot_block: Option<i64>,
    pub current_value_eth: Option<String>,
    pub pnl_eth: Option<String>,
    pub roi: Option<String>,
}

pub async fn analyze_strategy(pool: &PgPool, run_id: &str, limit: i64) -> Result<StrategyReport> {
    let run = load_run_metadata(pool, run_id).await?;
    let summary = load_summary(pool, run_id).await?;
    let concentration = load_concentration(pool, run_id).await?;
    let failures = load_failures(pool, run_id).await?;
    let buy_failed_entries =
        load_buy_failed_entries(pool, run_id, run.replay_run_id.as_deref(), limit).await?;
    let open_failed_exits = load_open_failed_exits(pool, run_id, limit).await?;
    let protocols = load_protocols(pool, run_id, run.replay_run_id.as_deref()).await?;
    let top_winners = load_ranked_positions(pool, run_id, limit, "DESC").await?;
    let worst_losers = load_ranked_positions(pool, run_id, limit, "ASC").await?;
    let issue_flags = issue_flags(&summary, &concentration, &run);

    Ok(StrategyReport {
        run,
        summary,
        concentration,
        issue_flags,
        failures,
        buy_failed_entries,
        open_failed_exits,
        protocols,
        top_winners,
        worst_losers,
    })
}

pub fn print_strategy_report(report: &StrategyReport) {
    println!("# Strategy Lab: {}", report.run.run_id);
    println!();
    println!(
        "- mode/status: `{}` / `{}`",
        report.run.mode, report.run.status
    );
    println!(
        "- replay run: `{}`",
        report
            .run
            .replay_run_id
            .as_deref()
            .unwrap_or("<none recorded>")
    );
    println!();

    println!("## Summary");
    render::print_table(
        &["metric", "value"],
        &[
            vec![
                "positions".to_string(),
                report.summary.positions.to_string(),
            ],
            vec![
                "open positions".to_string(),
                report.summary.open_positions.to_string(),
            ],
            vec![
                "failed positions".to_string(),
                report.summary.failed_positions.to_string(),
            ],
            vec![
                "buy failed positions".to_string(),
                report.summary.buy_failed_positions.to_string(),
            ],
            vec![
                "sell failed positions".to_string(),
                report.summary.sell_failed_positions.to_string(),
            ],
            vec![
                "entry cost ETH".to_string(),
                report.summary.entry_cost_eth.clone(),
            ],
            vec![
                "latest current value ETH".to_string(),
                report.summary.latest_current_value_eth.clone(),
            ],
            vec![
                "realized PnL ETH".to_string(),
                report.summary.realized_pnl_eth.clone(),
            ],
            vec![
                "unrealized PnL ETH".to_string(),
                report.summary.unrealized_pnl_eth.clone(),
            ],
            vec![
                "total PnL ETH".to_string(),
                report.summary.total_pnl_eth.clone(),
            ],
            vec![
                "ROI on open cost".to_string(),
                report
                    .summary
                    .total_roi_on_open_cost
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ],
            vec![
                "snapshots".to_string(),
                report.summary.snapshots.to_string(),
            ],
            vec![
                "positions with snapshots".to_string(),
                report.summary.snapshot_positions.to_string(),
            ],
            vec![
                "open without snapshot".to_string(),
                report.summary.open_without_snapshot.to_string(),
            ],
            vec![
                "zero decimal / nonzero raw".to_string(),
                report.summary.zero_decimal_nonzero_raw.to_string(),
            ],
        ],
    );
    println!();

    println!("## PnL Concentration");
    render::print_table(
        &["metric", "value"],
        &[
            vec![
                "snapshot positions".to_string(),
                report.concentration.snapshot_positions.to_string(),
            ],
            vec![
                "total PnL ETH".to_string(),
                report.concentration.total_pnl_eth.clone(),
            ],
            vec![
                "excluding top 1".to_string(),
                report.concentration.pnl_ex_top1_eth.clone(),
            ],
            vec![
                "excluding top 2".to_string(),
                report.concentration.pnl_ex_top2_eth.clone(),
            ],
            vec![
                "excluding top 5".to_string(),
                report.concentration.pnl_ex_top5_eth.clone(),
            ],
            vec![
                "excluding top 10".to_string(),
                report.concentration.pnl_ex_top10_eth.clone(),
            ],
        ],
    );
    println!();

    if !report.issue_flags.is_empty() {
        println!("## Flags");
        render::print_table(
            &["severity", "code", "message"],
            &report
                .issue_flags
                .iter()
                .map(|flag| {
                    vec![
                        flag.severity.clone(),
                        flag.code.clone(),
                        flag.message.clone(),
                    ]
                })
                .collect::<Vec<_>>(),
        );
        println!();
    }

    println!("## Protocols");
    render::print_table(
        &["protocol", "denom", "confirmed buys"],
        &report
            .protocols
            .iter()
            .map(|bucket| {
                vec![
                    bucket.protocol.clone(),
                    bucket.denom_symbol.clone(),
                    bucket.confirmed_buys.to_string(),
                ]
            })
            .collect::<Vec<_>>(),
    );
    println!();

    if !report.failures.is_empty() {
        println!("## Failure Buckets");
        render::print_table(
            &["error class", "reports"],
            &report
                .failures
                .iter()
                .map(|bucket| vec![bucket.error_class.clone(), bucket.reports.to_string()])
                .collect::<Vec<_>>(),
        );
        println!();
    }

    if !report.buy_failed_entries.is_empty() {
        println!("## Buy Failed Entries");
        render::print_table(
            &[
                "token", "block", "protocol", "denom", "obs buy", "obs sell", "error",
            ],
            &report
                .buy_failed_entries
                .iter()
                .map(|row| {
                    vec![
                        short(&row.token_address),
                        row.failed_block
                            .map(|block| block.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        row.protocol.clone(),
                        row.denom_symbol.clone(),
                        render_bool(row.observed_can_buy),
                        render_bool(row.observed_can_sell),
                        row.error_class.clone(),
                    ]
                })
                .collect::<Vec<_>>(),
        );
        println!();
    }

    if !report.open_failed_exits.is_empty() {
        println!("## Open Failed Exits");
        render::print_table(
            &[
                "token",
                "entry",
                "first fail",
                "last fail",
                "reports",
                "value",
                "pnl",
                "error",
            ],
            &report
                .open_failed_exits
                .iter()
                .map(|row| {
                    vec![
                        short(&row.token_address),
                        row.entry_block
                            .map(|block| block.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        row.first_failed_block
                            .map(|block| block.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        row.last_failed_block
                            .map(|block| block.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        row.failed_reports.to_string(),
                        render::fmt_opt(&row.current_value_eth),
                        render::fmt_opt(&row.pnl_eth),
                        row.error_class.clone(),
                    ]
                })
                .collect::<Vec<_>>(),
        );
        println!();
    }

    println!("## Top Winners");
    print_rank_table(&report.top_winners);
    println!();

    println!("## Worst Losers");
    print_rank_table(&report.worst_losers);
}

fn print_rank_table(rows: &[PositionRank]) {
    render::print_table(
        &["token", "entry", "snapshot", "cost", "value", "pnl", "roi"],
        &rows
            .iter()
            .map(|row| {
                vec![
                    short(&row.token_address),
                    row.entry_block
                        .map(|block| block.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    row.latest_snapshot_block
                        .map(|block| block.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    render::fmt_opt(&row.entry_cost_eth),
                    render::fmt_opt(&row.current_value_eth),
                    render::fmt_opt(&row.pnl_eth),
                    render::fmt_opt(&row.roi),
                ]
            })
            .collect::<Vec<_>>(),
    );
}

fn short(value: &str) -> String {
    if value.len() <= 14 {
        return value.to_string();
    }
    format!("{}...{}", &value[..8], &value[value.len() - 6..])
}

fn render_bool(value: Option<bool>) -> String {
    match value {
        Some(true) => "yes".to_string(),
        Some(false) => "no".to_string(),
        None => "-".to_string(),
    }
}

fn issue_flags(
    summary: &RunSummary,
    concentration: &PnlConcentration,
    run: &RunMetadata,
) -> Vec<IssueFlag> {
    let mut flags = Vec::new();
    if run.replay_run_id.is_none() && run.mode == "backtest" {
        flags.push(IssueFlag {
            severity: "medium".to_string(),
            code: "missing_replay_run_id".to_string(),
            message: "backtest run metadata has no replay_run_id; observation joins are incomplete"
                .to_string(),
        });
    }
    if summary.open_without_snapshot > 0 {
        flags.push(IssueFlag {
            severity: "high".to_string(),
            code: "open_without_snapshot".to_string(),
            message: format!(
                "{} open positions have no mark-to-market snapshot",
                summary.open_without_snapshot
            ),
        });
    }
    if summary.zero_decimal_nonzero_raw > 0 {
        flags.push(IssueFlag {
            severity: "high".to_string(),
            code: "zero_decimal_nonzero_raw".to_string(),
            message: format!(
                "{} positions have entry_token_amount=0 while raw token amount is nonzero",
                summary.zero_decimal_nonzero_raw
            ),
        });
    }
    if summary.buy_failed_positions > 0 {
        flags.push(IssueFlag {
            severity: "medium".to_string(),
            code: "buy_failed_positions".to_string(),
            message: format!(
                "{} positions failed entry simulation",
                summary.buy_failed_positions
            ),
        });
    }
    if summary.sell_failed_positions > 0 {
        flags.push(IssueFlag {
            severity: "high".to_string(),
            code: "sell_failed_positions".to_string(),
            message: format!(
                "{} positions still have exposure after a failed sell",
                summary.sell_failed_positions
            ),
        });
    }
    if concentration.total_pnl_eth != "0" && concentration.pnl_ex_top1_eth.starts_with('-') {
        flags.push(IssueFlag {
            severity: "medium".to_string(),
            code: "top1_concentration".to_string(),
            message: "run-level PnL turns negative after excluding the top winner".to_string(),
        });
    }
    flags
}

async fn load_summary(pool: &PgPool, run_id: &str) -> Result<RunSummary> {
    let row = sqlx::query(
        r#"
        WITH latest AS (
            SELECT DISTINCT ON (ps.run_id, ps.position_id)
                   ps.run_id,
                   ps.position_id,
                   NULLIF(ps.current_value_eth, '')::numeric AS current_value_eth,
                   NULLIF(ps.realized_profit_eth, '')::numeric AS realized_profit_eth,
                   NULLIF(ps.unrealized_profit_eth, '')::numeric AS unrealized_profit_eth
            FROM alpha_trading.position_snapshots ps
            WHERE ps.run_id = $1
            ORDER BY ps.run_id, ps.position_id, ps.block_number DESC NULLS LAST, ps.id DESC
        ),
        pos AS (
            SELECT run_id, position_id, state, payload
            FROM alpha_trading.positions
            WHERE run_id = $1
        ),
        execs AS (
            SELECT
                count(*) AS execution_reports,
                count(*) FILTER (WHERE status = 'confirmed') AS confirmed_reports,
                count(*) FILTER (WHERE status = 'failed') AS failed_reports
            FROM alpha_trading.execution_reports
            WHERE run_id = $1
        ),
        snaps AS (
            SELECT count(*) AS snapshots, count(DISTINCT position_id) AS snapshot_positions
            FROM alpha_trading.position_snapshots
            WHERE run_id = $1
        ),
        rollup AS (
            SELECT
                count(*) AS positions,
                count(*) FILTER (
                    WHERE state IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                ) AS open_positions,
                count(*) FILTER (WHERE state IN ('buy_failed', 'sell_failed')) AS failed_positions,
                count(*) FILTER (WHERE state='buy_failed') AS buy_failed_positions,
                count(*) FILTER (WHERE state='sell_failed') AS sell_failed_positions,
                coalesce(sum((payload->>'entry_cost_basis')::numeric), 0) AS entry_cost_eth,
                count(l.*) AS latest_snapshot_positions,
                coalesce(sum(l.current_value_eth), 0) AS latest_current_value_eth,
                coalesce(sum(l.realized_profit_eth), 0) AS realized_pnl_eth,
                coalesce(sum(l.unrealized_profit_eth), 0) AS unrealized_pnl_eth,
                coalesce(sum(l.realized_profit_eth + l.unrealized_profit_eth), 0) AS total_pnl_eth,
                count(*) FILTER (
                    WHERE state IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                      AND l.position_id IS NULL
                ) AS open_without_snapshot,
                count(*) FILTER (
                    WHERE state IN ('buy_confirmed', 'sell_failed', 'sell_cancelled')
                      AND (payload->>'entry_token_amount')::numeric = 0
                      AND coalesce(payload->'entry_token_raw_amount'->>'raw', '') NOT IN ('', '0', '0x', '0x0')
                ) AS zero_decimal_nonzero_raw,
                CASE
                    WHEN sum((payload->>'entry_cost_basis')::numeric) FILTER (
                        WHERE state IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                    ) > 0
                    THEN coalesce(sum(l.realized_profit_eth + l.unrealized_profit_eth), 0)
                         / (sum((payload->>'entry_cost_basis')::numeric) FILTER (
                             WHERE state IN ('buy_confirmed', 'sell_intent_created', 'sell_submitted', 'sell_failed', 'sell_cancelled')
                         ))
                    ELSE NULL
                END AS total_roi_on_open_cost
            FROM pos
            LEFT JOIN latest l USING (run_id, position_id)
        )
        SELECT
            rollup.positions,
            rollup.open_positions,
            rollup.failed_positions,
            rollup.buy_failed_positions,
            rollup.sell_failed_positions,
            rollup.entry_cost_eth::text AS entry_cost_eth,
            rollup.latest_current_value_eth::text AS latest_current_value_eth,
            rollup.realized_pnl_eth::text AS realized_pnl_eth,
            rollup.unrealized_pnl_eth::text AS unrealized_pnl_eth,
            rollup.total_pnl_eth::text AS total_pnl_eth,
            rollup.total_roi_on_open_cost::text AS total_roi_on_open_cost,
            rollup.open_without_snapshot,
            rollup.zero_decimal_nonzero_raw,
            execs.execution_reports,
            execs.confirmed_reports,
            execs.failed_reports,
            snaps.snapshots,
            snaps.snapshot_positions
        FROM rollup, execs, snaps
        "#,
    )
    .bind(run_id)
    .fetch_one(pool)
    .await
    .wrap_err("failed to load strategy summary")?;

    Ok(RunSummary {
        positions: row.try_get("positions")?,
        open_positions: row.try_get("open_positions")?,
        failed_positions: row.try_get("failed_positions")?,
        buy_failed_positions: row.try_get("buy_failed_positions")?,
        sell_failed_positions: row.try_get("sell_failed_positions")?,
        entry_cost_eth: row.try_get("entry_cost_eth")?,
        execution_reports: row.try_get("execution_reports")?,
        confirmed_reports: row.try_get("confirmed_reports")?,
        failed_reports: row.try_get("failed_reports")?,
        snapshots: row.try_get("snapshots")?,
        snapshot_positions: row.try_get("snapshot_positions")?,
        open_without_snapshot: row.try_get("open_without_snapshot")?,
        zero_decimal_nonzero_raw: row.try_get("zero_decimal_nonzero_raw")?,
        latest_current_value_eth: row.try_get("latest_current_value_eth")?,
        realized_pnl_eth: row.try_get("realized_pnl_eth")?,
        unrealized_pnl_eth: row.try_get("unrealized_pnl_eth")?,
        total_pnl_eth: row.try_get("total_pnl_eth")?,
        total_roi_on_open_cost: row.try_get("total_roi_on_open_cost")?,
    })
}

async fn load_concentration(pool: &PgPool, run_id: &str) -> Result<PnlConcentration> {
    let row = sqlx::query(
        r#"
        WITH latest AS (
            SELECT DISTINCT ON (ps.run_id, ps.position_id)
                   NULLIF(ps.realized_profit_eth, '')::numeric
                     + NULLIF(ps.unrealized_profit_eth, '')::numeric AS pnl
            FROM alpha_trading.position_snapshots ps
            WHERE ps.run_id = $1
            ORDER BY ps.run_id, ps.position_id, ps.block_number DESC NULLS LAST, ps.id DESC
        ),
        ranked AS (
            SELECT pnl, row_number() OVER (ORDER BY pnl DESC) AS rn
            FROM latest
        )
        SELECT
            count(*) AS snapshot_positions,
            coalesce(sum(pnl), 0)::text AS total_pnl_eth,
            coalesce(sum(pnl) FILTER (WHERE rn > 1), 0)::text AS pnl_ex_top1_eth,
            coalesce(sum(pnl) FILTER (WHERE rn > 2), 0)::text AS pnl_ex_top2_eth,
            coalesce(sum(pnl) FILTER (WHERE rn > 5), 0)::text AS pnl_ex_top5_eth,
            coalesce(sum(pnl) FILTER (WHERE rn > 10), 0)::text AS pnl_ex_top10_eth
        FROM ranked
        "#,
    )
    .bind(run_id)
    .fetch_one(pool)
    .await
    .wrap_err("failed to load PnL concentration")?;

    Ok(PnlConcentration {
        snapshot_positions: row.try_get("snapshot_positions")?,
        total_pnl_eth: row.try_get("total_pnl_eth")?,
        pnl_ex_top1_eth: row.try_get("pnl_ex_top1_eth")?,
        pnl_ex_top2_eth: row.try_get("pnl_ex_top2_eth")?,
        pnl_ex_top5_eth: row.try_get("pnl_ex_top5_eth")?,
        pnl_ex_top10_eth: row.try_get("pnl_ex_top10_eth")?,
    })
}

async fn load_failures(pool: &PgPool, run_id: &str) -> Result<Vec<FailureBucket>> {
    let rows = sqlx::query(
        r#"
        SELECT
            CASE
                WHEN error LIKE 'invalid pool parameters for chain simulation: pool % has no denomination address'
                    THEN 'missing denom address'
                WHEN error LIKE 'Universal Router V4 buy transaction failed%'
                    THEN 'v4 universal router buy reverted'
                WHEN error = 'Buy transaction failed'
                    THEN 'buy tx failed'
                WHEN error LIKE 'invalid pool parameters for chain simulation: protocol % not supported by chain simulator'
                    THEN 'unsupported protocol'
                WHEN error LIKE 'chain buy simulation failed:%'
                    THEN 'chain buy simulation failed'
                ELSE coalesce(error, '<none>')
            END AS error_class,
            count(*) AS reports
        FROM alpha_trading.execution_reports
        WHERE run_id = $1 AND status = 'failed'
        GROUP BY error_class
        ORDER BY reports DESC, error_class
        "#,
    )
    .bind(run_id)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load failure buckets")?;

    rows.into_iter()
        .map(|row| {
            Ok(FailureBucket {
                error_class: row.try_get("error_class")?,
                reports: row.try_get("reports")?,
            })
        })
        .collect()
}

async fn load_buy_failed_entries(
    pool: &PgPool,
    run_id: &str,
    replay_run_id: Option<&str>,
    limit: i64,
) -> Result<Vec<BuyFailedEntry>> {
    let rows = sqlx::query(
        r#"
        WITH failed AS (
            SELECT run_id, position_id, block_number, error
            FROM alpha_trading.execution_reports
            WHERE run_id = $1
              AND order_side = 'buy'
              AND status = 'failed'
        )
        SELECT
            p.token_address,
            p.pool_address,
            f.block_number AS failed_block,
            coalesce(so.payload->'pool'->>'protocol', '<no observation>') AS protocol,
            coalesce(so.payload->'pool'->>'denom_symbol', so.payload->'pool'->>'currency', '<missing>') AS denom_symbol,
            (so.payload->'pool'->>'can_buy')::boolean AS can_buy,
            (so.payload->'pool'->>'can_sell')::boolean AS can_sell,
            CASE
                WHEN f.error LIKE 'Universal Router V4 buy transaction failed%'
                    THEN 'v4 universal router buy reverted'
                WHEN f.error LIKE 'Buy transaction failed:%'
                    THEN replace(f.error, 'Buy transaction failed: ', '')
                ELSE coalesce(left(f.error, 96), '<none>')
            END AS error_class
        FROM failed f
        JOIN alpha_trading.positions p USING (run_id, position_id)
        LEFT JOIN alpha_trading.strategy_observations so
          ON so.run_id = $2
         AND so.event_source = 'pool_update'
         AND so.block_number = f.block_number
         AND lower(so.token_address) = lower(p.token_address)
         AND so.pool_address = p.pool_address
        WHERE p.run_id = $1
        ORDER BY f.block_number, p.token_address, p.pool_address
        LIMIT $3
        "#,
    )
    .bind(run_id)
    .bind(replay_run_id.unwrap_or(""))
    .bind(limit)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load buy failed entries")?;

    rows.into_iter()
        .map(|row| {
            Ok(BuyFailedEntry {
                token_address: row.try_get("token_address")?,
                pool_address: row.try_get("pool_address")?,
                failed_block: row.try_get("failed_block")?,
                protocol: row.try_get("protocol")?,
                denom_symbol: row.try_get("denom_symbol")?,
                observed_can_buy: row.try_get("can_buy")?,
                observed_can_sell: row.try_get("can_sell")?,
                error_class: row.try_get("error_class")?,
            })
        })
        .collect()
}

async fn load_open_failed_exits(
    pool: &PgPool,
    run_id: &str,
    limit: i64,
) -> Result<Vec<OpenFailedExit>> {
    let rows = sqlx::query(
        r#"
        WITH latest AS (
            SELECT DISTINCT ON (ps.run_id, ps.position_id)
                   ps.run_id,
                   ps.position_id,
                   NULLIF(ps.current_value_eth, '')::numeric AS current_value_eth,
                   NULLIF(ps.realized_profit_eth, '')::numeric
                     + NULLIF(ps.unrealized_profit_eth, '')::numeric AS pnl,
                   ps.block_number AS snapshot_block
            FROM alpha_trading.position_snapshots ps
            WHERE ps.run_id = $1
            ORDER BY ps.run_id, ps.position_id, ps.block_number DESC NULLS LAST, ps.id DESC
        ),
        failed AS (
            SELECT
                run_id,
                position_id,
                count(*) AS failed_reports,
                min(block_number) AS first_failed_block,
                max(block_number) AS last_failed_block,
                string_agg(
                    DISTINCT CASE
                        WHEN error LIKE '%TRANSFER_FROM_FAILED%' THEN 'TRANSFER_FROM_FAILED'
                        WHEN error LIKE '%Empty revert payload%' THEN 'empty_revert'
                        ELSE coalesce(left(error, 80), '<none>')
                    END,
                    ' | '
                ) AS error_class
            FROM alpha_trading.execution_reports
            WHERE run_id = $1
              AND order_side = 'sell'
              AND status = 'failed'
            GROUP BY run_id, position_id
        )
        SELECT
            p.token_address,
            p.pool_address,
            (p.payload->>'entry_block')::bigint AS entry_block,
            f.failed_reports,
            f.first_failed_block,
            f.last_failed_block,
            l.snapshot_block,
            l.current_value_eth::text AS current_value_eth,
            l.pnl::text AS pnl_eth,
            f.error_class
        FROM alpha_trading.positions p
        JOIN failed f USING (run_id, position_id)
        LEFT JOIN latest l USING (run_id, position_id)
        WHERE p.run_id = $1
          AND p.state = 'sell_failed'
        ORDER BY f.first_failed_block, p.token_address, p.pool_address
        LIMIT $2
        "#,
    )
    .bind(run_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load open failed exits")?;

    rows.into_iter()
        .map(|row| {
            Ok(OpenFailedExit {
                token_address: row.try_get("token_address")?,
                pool_address: row.try_get("pool_address")?,
                entry_block: row.try_get("entry_block")?,
                failed_reports: row.try_get("failed_reports")?,
                first_failed_block: row.try_get("first_failed_block")?,
                last_failed_block: row.try_get("last_failed_block")?,
                latest_snapshot_block: row.try_get("snapshot_block")?,
                current_value_eth: row.try_get("current_value_eth")?,
                pnl_eth: row.try_get("pnl_eth")?,
                error_class: row.try_get("error_class")?,
            })
        })
        .collect()
}

async fn load_protocols(
    pool: &PgPool,
    run_id: &str,
    replay_run_id: Option<&str>,
) -> Result<Vec<ProtocolBucket>> {
    let Some(replay_run_id) = replay_run_id else {
        return Ok(Vec::new());
    };
    let rows = sqlx::query(
        r#"
        WITH pos AS (
            SELECT lower(pool_address) AS pool_id, (payload->>'entry_block')::bigint AS entry_block
            FROM alpha_trading.positions
            WHERE run_id = $1
              AND payload ? 'entry_block'
              AND payload->>'entry_block' IS NOT NULL
              AND state <> 'buy_failed'
        )
        SELECT
            coalesce(so.payload->'pool'->>'protocol', '<no observation>') AS protocol,
            coalesce(so.payload->'pool'->>'denom_symbol', so.payload->'pool'->>'currency', '<missing>') AS denom_symbol,
            count(*) AS confirmed_buys
        FROM pos
        LEFT JOIN alpha_trading.strategy_observations so
          ON so.run_id = $2
         AND so.event_source = 'pool_update'
         AND so.block_number = pos.entry_block
         AND lower((so.payload->'pool'->>'token_address') || ':' || (so.payload->'pool'->>'pool_address')) = pos.pool_id
        GROUP BY protocol, denom_symbol
        ORDER BY confirmed_buys DESC, protocol, denom_symbol
        "#,
    )
    .bind(run_id)
    .bind(replay_run_id)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load protocol buckets")?;

    rows.into_iter()
        .map(|row| {
            Ok(ProtocolBucket {
                protocol: row.try_get("protocol")?,
                denom_symbol: row.try_get("denom_symbol")?,
                confirmed_buys: row.try_get("confirmed_buys")?,
            })
        })
        .collect()
}

async fn load_ranked_positions(
    pool: &PgPool,
    run_id: &str,
    limit: i64,
    direction: &str,
) -> Result<Vec<PositionRank>> {
    let order = match direction {
        "ASC" => "ASC",
        _ => "DESC",
    };
    let query = format!(
        r#"
        WITH latest AS (
            SELECT DISTINCT ON (ps.run_id, ps.position_id)
                   ps.run_id,
                   ps.position_id,
                   NULLIF(ps.current_value_eth, '')::numeric AS current_value_eth,
                   NULLIF(ps.realized_profit_eth, '')::numeric
                     + NULLIF(ps.unrealized_profit_eth, '')::numeric AS pnl,
                   NULLIF(ps.roi, '')::numeric AS roi,
                   ps.block_number AS snapshot_block
            FROM alpha_trading.position_snapshots ps
            WHERE ps.run_id = $1
            ORDER BY ps.run_id, ps.position_id, ps.block_number DESC NULLS LAST, ps.id DESC
        )
        SELECT
            p.token_address,
            p.pool_address,
            (p.payload->>'entry_block')::bigint AS entry_block,
            (p.payload->>'entry_cost_basis') AS entry_cost_eth,
            (p.payload->>'entry_token_amount') AS entry_token_amount,
            l.snapshot_block,
            l.current_value_eth::text AS current_value_eth,
            l.pnl::text AS pnl_eth,
            l.roi::text AS roi
        FROM alpha_trading.positions p
        JOIN latest l USING (run_id, position_id)
        WHERE p.run_id = $1
        ORDER BY l.pnl {order}
        LIMIT $2
        "#
    );
    let rows = sqlx::query(&query)
        .bind(run_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .wrap_err("failed to load ranked positions")?;

    rows.into_iter().map(position_rank_from_row).collect()
}

fn position_rank_from_row(row: sqlx::postgres::PgRow) -> Result<PositionRank> {
    Ok(PositionRank {
        token_address: row.try_get("token_address")?,
        pool_address: row.try_get("pool_address")?,
        entry_block: row.try_get("entry_block")?,
        entry_cost_eth: row.try_get("entry_cost_eth")?,
        entry_token_amount: row.try_get("entry_token_amount")?,
        latest_snapshot_block: row.try_get("snapshot_block")?,
        current_value_eth: row.try_get("current_value_eth")?,
        pnl_eth: row.try_get("pnl_eth")?,
        roi: row.try_get("roi")?,
    })
}
