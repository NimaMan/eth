use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

use crate::{is_nonzero_hex, load_run_metadata, parse_f64, render, RunMetadata};

#[derive(Clone, Debug)]
pub enum PositionSelector {
    PositionId(String),
    Token(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionReport {
    pub run: RunMetadata,
    pub position: PositionRecord,
    pub entry_report: Option<ExecutionReportRecord>,
    pub latest_snapshot: Option<SnapshotRecord>,
    pub entry_observation: Option<PoolObservation>,
    pub latest_observation: Option<PoolObservation>,
    pub trajectory: Vec<TrajectoryPoint>,
    pub checks: Vec<PositionCheck>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionRecord {
    pub run_id: String,
    pub position_id: String,
    pub token_address: String,
    pub pool_address: String,
    pub state: String,
    pub entry_order_id: Option<String>,
    pub entry_block: Option<i64>,
    pub entry_cost_eth: Option<String>,
    pub entry_token_amount: Option<String>,
    pub entry_token_raw: Option<String>,
    pub entry_token_decimals: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionReportRecord {
    pub order_id: String,
    pub status: String,
    pub block_number: Option<i64>,
    pub filled_amount_raw: Option<String>,
    pub filled_amount_decimals: Option<i16>,
    pub gas_used: Option<i64>,
    pub error: Option<String>,
    pub token_amount_raw: Option<String>,
    pub token_amount_decimals: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub block_number: Option<i64>,
    pub current_value_eth: Option<String>,
    pub realized_profit_eth: Option<String>,
    pub unrealized_profit_eth: Option<String>,
    pub roi: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolObservation {
    pub block_number: Option<i64>,
    pub protocol: Option<String>,
    pub denom_symbol: Option<String>,
    pub denom_reserve: Option<String>,
    pub token_reserve: Option<String>,
    pub price: Option<String>,
    pub can_buy: Option<bool>,
    pub can_sell: Option<bool>,
    pub is_scam: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrajectoryPoint {
    pub block_number: Option<i64>,
    pub current_value_eth: Option<String>,
    pub unrealized_profit_eth: Option<String>,
    pub roi: Option<String>,
    pub denom_reserve: Option<String>,
    pub token_reserve: Option<String>,
    pub spot_price: Option<String>,
    pub can_buy: Option<bool>,
    pub can_sell: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionCheck {
    pub status: String,
    pub code: String,
    pub message: String,
}

pub async fn analyze_position(
    pool: &PgPool,
    run_id: &str,
    selector: PositionSelector,
    replay_override: Option<&str>,
    samples: i64,
) -> Result<PositionReport> {
    let mut run = load_run_metadata(pool, run_id).await?;
    if let Some(replay_run_id) = replay_override {
        run.replay_run_id = Some(replay_run_id.to_string());
    }

    let position = load_position(pool, run_id, selector).await?;
    let entry_report = load_entry_report(pool, &position).await?;
    let latest_snapshot = load_latest_snapshot(pool, &position).await?;

    let replay_run_id = run.replay_run_id.as_deref();
    let entry_observation = match (replay_run_id, position.entry_block) {
        (Some(replay), Some(block)) => {
            load_pool_observation(pool, replay, block, &position.pool_address).await?
        }
        _ => None,
    };
    let latest_observation = match (
        replay_run_id,
        latest_snapshot.as_ref().and_then(|s| s.block_number),
    ) {
        (Some(replay), Some(block)) => {
            load_pool_observation(pool, replay, block, &position.pool_address).await?
        }
        _ => None,
    };
    let trajectory =
        load_trajectory(pool, &position, replay_run_id.unwrap_or(""), samples.max(1)).await?;
    let checks = build_checks(
        &run,
        &position,
        entry_report.as_ref(),
        latest_snapshot.as_ref(),
        entry_observation.as_ref(),
        latest_observation.as_ref(),
    );

    Ok(PositionReport {
        run,
        position,
        entry_report,
        latest_snapshot,
        entry_observation,
        latest_observation,
        trajectory,
        checks,
    })
}

pub fn print_position_report(report: &PositionReport) {
    println!("# Token Lab: {}", report.position.token_address);
    println!();
    println!("- run: `{}`", report.position.run_id);
    println!(
        "- replay run: `{}`",
        report
            .run
            .replay_run_id
            .as_deref()
            .unwrap_or("<none recorded>")
    );
    println!("- pool: `{}`", report.position.pool_address);
    println!();

    println!("## Position");
    render::print_table(
        &["field", "value"],
        &[
            vec!["state".to_string(), report.position.state.clone()],
            vec![
                "entry order".to_string(),
                report
                    .position
                    .entry_order_id
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ],
            vec![
                "entry block".to_string(),
                report
                    .position
                    .entry_block
                    .map(|block| block.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ],
            vec![
                "entry cost ETH".to_string(),
                render::fmt_opt(&report.position.entry_cost_eth),
            ],
            vec![
                "entry token amount".to_string(),
                render::fmt_opt(&report.position.entry_token_amount),
            ],
            vec![
                "entry token raw".to_string(),
                render::fmt_opt(&report.position.entry_token_raw),
            ],
        ],
    );
    println!();

    if let Some(report_row) = &report.entry_report {
        println!("## Entry Report");
        render::print_table(
            &["field", "value"],
            &[
                vec!["status".to_string(), report_row.status.clone()],
                vec![
                    "block".to_string(),
                    report_row
                        .block_number
                        .map(|block| block.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ],
                vec![
                    "gas".to_string(),
                    report_row
                        .gas_used
                        .map(|gas| gas.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ],
                vec![
                    "filled raw".to_string(),
                    render::fmt_opt(&report_row.filled_amount_raw),
                ],
                vec![
                    "token raw".to_string(),
                    render::fmt_opt(&report_row.token_amount_raw),
                ],
                vec!["error".to_string(), render::fmt_opt(&report_row.error)],
            ],
        );
        println!();
    }

    println!("## Observations");
    render::print_table(
        &[
            "point",
            "block",
            "protocol",
            "denom",
            "denom reserve",
            "token reserve",
            "price",
            "can buy",
            "can sell",
        ],
        &[
            observation_row("entry", report.entry_observation.as_ref()),
            observation_row("latest", report.latest_observation.as_ref()),
        ],
    );
    println!();

    println!("## Latest Snapshot");
    if let Some(snapshot) = &report.latest_snapshot {
        render::print_table(
            &["field", "value"],
            &[
                vec![
                    "block".to_string(),
                    snapshot
                        .block_number
                        .map(|block| block.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ],
                vec![
                    "current value ETH".to_string(),
                    render::fmt_opt(&snapshot.current_value_eth),
                ],
                vec![
                    "unrealized PnL ETH".to_string(),
                    render::fmt_opt(&snapshot.unrealized_profit_eth),
                ],
                vec!["ROI".to_string(), render::fmt_opt(&snapshot.roi)],
            ],
        );
    } else {
        println!("No snapshots for this position.");
    }
    println!();

    println!("## Checks");
    render::print_table(
        &["status", "code", "message"],
        &report
            .checks
            .iter()
            .map(|check| {
                vec![
                    check.status.clone(),
                    check.code.clone(),
                    check.message.clone(),
                ]
            })
            .collect::<Vec<_>>(),
    );
    println!();

    println!("## Trajectory");
    render::print_table(
        &[
            "block",
            "value ETH",
            "PnL ETH",
            "ROI",
            "denom reserve",
            "token reserve",
            "spot",
            "buy",
            "sell",
        ],
        &report
            .trajectory
            .iter()
            .map(|point| {
                vec![
                    point
                        .block_number
                        .map(|block| block.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    render::fmt_opt(&point.current_value_eth),
                    render::fmt_opt(&point.unrealized_profit_eth),
                    render::fmt_opt(&point.roi),
                    render::fmt_opt(&point.denom_reserve),
                    render::fmt_opt(&point.token_reserve),
                    render::fmt_opt(&point.spot_price),
                    point
                        .can_buy
                        .map(render::fmt_bool)
                        .unwrap_or("-")
                        .to_string(),
                    point
                        .can_sell
                        .map(render::fmt_bool)
                        .unwrap_or("-")
                        .to_string(),
                ]
            })
            .collect::<Vec<_>>(),
    );
}

fn observation_row(label: &str, observation: Option<&PoolObservation>) -> Vec<String> {
    if let Some(obs) = observation {
        vec![
            label.to_string(),
            obs.block_number
                .map(|block| block.to_string())
                .unwrap_or_else(|| "-".to_string()),
            render::fmt_opt(&obs.protocol),
            render::fmt_opt(&obs.denom_symbol),
            render::fmt_opt(&obs.denom_reserve),
            render::fmt_opt(&obs.token_reserve),
            render::fmt_opt(&obs.price),
            obs.can_buy.map(render::fmt_bool).unwrap_or("-").to_string(),
            obs.can_sell
                .map(render::fmt_bool)
                .unwrap_or("-")
                .to_string(),
        ]
    } else {
        vec![
            label.to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
        ]
    }
}

fn build_checks(
    run: &RunMetadata,
    position: &PositionRecord,
    entry_report: Option<&ExecutionReportRecord>,
    latest_snapshot: Option<&SnapshotRecord>,
    entry_observation: Option<&PoolObservation>,
    latest_observation: Option<&PoolObservation>,
) -> Vec<PositionCheck> {
    let mut checks = Vec::new();

    push_check(
        &mut checks,
        run.replay_run_id.is_some(),
        "replay_run_recorded",
        "run metadata records replay_run_id for observation joins",
        "run metadata has no replay_run_id; pass --replay-run-id for observation joins",
    );

    match entry_report {
        Some(report) => {
            push_check(
                &mut checks,
                report.status == "confirmed",
                "entry_confirmed",
                "entry execution report is confirmed",
                "entry execution report is not confirmed",
            );
            if let (Some(report_block), Some(entry_block)) =
                (report.block_number, position.entry_block)
            {
                push_check(
                    &mut checks,
                    report_block == entry_block,
                    "entry_block_match",
                    "entry report block matches position entry_block",
                    "entry report block does not match position entry_block",
                );
            }
            push_check(
                &mut checks,
                report.filled_amount_raw.is_some(),
                "entry_cost_recorded",
                "entry report has filled ETH amount",
                "entry report is missing filled ETH amount",
            );
            push_check(
                &mut checks,
                report
                    .token_amount_raw
                    .as_deref()
                    .map(|raw| is_nonzero_hex(Some(raw)))
                    .unwrap_or(false),
                "entry_token_raw_nonzero",
                "entry report has nonzero raw token amount",
                "entry report raw token amount is missing or zero",
            );
        }
        None => checks.push(PositionCheck {
            status: "fail".to_string(),
            code: "entry_report_missing".to_string(),
            message: "position entry_order_id has no execution report".to_string(),
        }),
    }

    let decimal_zero = position
        .entry_token_amount
        .as_deref()
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value == 0.0)
        .unwrap_or(false);
    let raw_nonzero = is_nonzero_hex(position.entry_token_raw.as_deref());
    if decimal_zero && raw_nonzero {
        checks.push(PositionCheck {
            status: "warn".to_string(),
            code: "zero_decimal_nonzero_raw".to_string(),
            message:
                "entry_token_amount is zero but entry_token_raw_amount is nonzero; decimal display likely overflowed"
                    .to_string(),
        });
    } else {
        checks.push(PositionCheck {
            status: "pass".to_string(),
            code: "decimal_raw_consistency".to_string(),
            message: "decimal token amount is consistent with raw amount".to_string(),
        });
    }

    push_check(
        &mut checks,
        latest_snapshot.is_some(),
        "has_snapshot",
        "position has at least one mark-to-market snapshot",
        "position has no mark-to-market snapshot",
    );

    match entry_observation {
        Some(obs) => {
            push_check(
                &mut checks,
                obs.can_buy == Some(true) && obs.can_sell == Some(true),
                "entry_tradable",
                "entry observation is buyable and sellable",
                "entry observation is not buyable and sellable",
            );
            let denom_reserve = parse_f64(&obs.denom_reserve);
            push_check(
                &mut checks,
                denom_reserve.map(|reserve| reserve >= 0.5).unwrap_or(false),
                "entry_liquidity_threshold",
                "entry denom reserve is at or above 0.5 ETH/WETH",
                "entry denom reserve is below threshold or unavailable",
            );
        }
        None => checks.push(PositionCheck {
            status: "warn".to_string(),
            code: "entry_observation_missing".to_string(),
            message: "could not find matching entry pool observation".to_string(),
        }),
    }

    if let (Some(snapshot), Some(obs)) = (latest_snapshot, latest_observation) {
        let value = parse_f64(&snapshot.current_value_eth).unwrap_or_default();
        if value == 0.0 && obs.can_sell == Some(false) {
            checks.push(PositionCheck {
                status: "pass".to_string(),
                code: "zero_value_explained_by_untradable_pool".to_string(),
                message: "latest value is zero and latest observation is not sellable".to_string(),
            });
        }
    }

    checks
}

fn push_check(
    checks: &mut Vec<PositionCheck>,
    passed: bool,
    code: &str,
    pass_message: &str,
    fail_message: &str,
) {
    checks.push(PositionCheck {
        status: if passed { "pass" } else { "fail" }.to_string(),
        code: code.to_string(),
        message: if passed { pass_message } else { fail_message }.to_string(),
    });
}

async fn load_position(
    pool: &PgPool,
    run_id: &str,
    selector: PositionSelector,
) -> Result<PositionRecord> {
    let (query, value) = match selector {
        PositionSelector::PositionId(position_id) => (
            r#"
            SELECT run_id, position_id, token_address, pool_address, state, entry_order_id,
                   (payload->>'entry_block')::bigint AS entry_block,
                   payload->>'entry_cost_basis' AS entry_cost_eth,
                   payload->>'entry_token_amount' AS entry_token_amount,
                   payload->'entry_token_raw_amount'->>'raw' AS entry_token_raw,
                   (payload->'entry_token_raw_amount'->>'decimals')::bigint AS entry_token_decimals
            FROM alpha_trading.positions
            WHERE run_id = $1 AND position_id = $2
            "#,
            position_id,
        ),
        PositionSelector::Token(token) => (
            r#"
            SELECT run_id, position_id, token_address, pool_address, state, entry_order_id,
                   (payload->>'entry_block')::bigint AS entry_block,
                   payload->>'entry_cost_basis' AS entry_cost_eth,
                   payload->>'entry_token_amount' AS entry_token_amount,
                   payload->'entry_token_raw_amount'->>'raw' AS entry_token_raw,
                   (payload->'entry_token_raw_amount'->>'decimals')::bigint AS entry_token_decimals
            FROM alpha_trading.positions
            WHERE run_id = $1 AND lower(token_address) = lower($2)
            "#,
            token,
        ),
    };

    let rows = sqlx::query(query)
        .bind(run_id)
        .bind(value)
        .fetch_all(pool)
        .await
        .wrap_err("failed to load position")?;

    match rows.len() {
        0 => Err(eyre::eyre!("no position matched selector")),
        1 => position_from_row(&rows[0]),
        count => Err(eyre::eyre!(
            "{count} positions matched selector; use --position-id"
        )),
    }
}

fn position_from_row(row: &sqlx::postgres::PgRow) -> Result<PositionRecord> {
    Ok(PositionRecord {
        run_id: row.try_get("run_id")?,
        position_id: row.try_get("position_id")?,
        token_address: row.try_get("token_address")?,
        pool_address: row.try_get("pool_address")?,
        state: row.try_get("state")?,
        entry_order_id: row.try_get("entry_order_id")?,
        entry_block: row.try_get("entry_block")?,
        entry_cost_eth: row.try_get("entry_cost_eth")?,
        entry_token_amount: row.try_get("entry_token_amount")?,
        entry_token_raw: row.try_get("entry_token_raw")?,
        entry_token_decimals: row.try_get("entry_token_decimals")?,
    })
}

async fn load_entry_report(
    pool: &PgPool,
    position: &PositionRecord,
) -> Result<Option<ExecutionReportRecord>> {
    let Some(order_id) = &position.entry_order_id else {
        return Ok(None);
    };
    let row = sqlx::query(
        r#"
        SELECT order_id, status, block_number, filled_amount_raw, filled_amount_decimals,
               gas_used, error,
               payload->'token_amount'->>'raw' AS token_amount_raw,
               (payload->'token_amount'->>'decimals')::bigint AS token_amount_decimals
        FROM alpha_trading.execution_reports
        WHERE run_id = $1 AND order_id = $2
        "#,
    )
    .bind(&position.run_id)
    .bind(order_id)
    .fetch_optional(pool)
    .await
    .wrap_err("failed to load entry execution report")?;

    row.map(|row| {
        Ok(ExecutionReportRecord {
            order_id: row.try_get("order_id")?,
            status: row.try_get("status")?,
            block_number: row.try_get("block_number")?,
            filled_amount_raw: row.try_get("filled_amount_raw")?,
            filled_amount_decimals: row.try_get("filled_amount_decimals")?,
            gas_used: row.try_get("gas_used")?,
            error: row.try_get("error")?,
            token_amount_raw: row.try_get("token_amount_raw")?,
            token_amount_decimals: row.try_get("token_amount_decimals")?,
        })
    })
    .transpose()
}

async fn load_latest_snapshot(
    pool: &PgPool,
    position: &PositionRecord,
) -> Result<Option<SnapshotRecord>> {
    let row = sqlx::query(
        r#"
        SELECT block_number, current_value_eth, realized_profit_eth, unrealized_profit_eth, roi
        FROM alpha_trading.position_snapshots
        WHERE run_id = $1 AND position_id = $2
        ORDER BY block_number DESC NULLS LAST, id DESC
        LIMIT 1
        "#,
    )
    .bind(&position.run_id)
    .bind(&position.position_id)
    .fetch_optional(pool)
    .await
    .wrap_err("failed to load latest position snapshot")?;

    row.map(|row| snapshot_from_row(&row)).transpose()
}

fn snapshot_from_row(row: &sqlx::postgres::PgRow) -> Result<SnapshotRecord> {
    Ok(SnapshotRecord {
        block_number: row.try_get("block_number")?,
        current_value_eth: row.try_get("current_value_eth")?,
        realized_profit_eth: row.try_get("realized_profit_eth")?,
        unrealized_profit_eth: row.try_get("unrealized_profit_eth")?,
        roi: row.try_get("roi")?,
    })
}

async fn load_pool_observation(
    pool: &PgPool,
    replay_run_id: &str,
    block_number: i64,
    pool_id: &str,
) -> Result<Option<PoolObservation>> {
    let row = sqlx::query(
        r#"
        SELECT block_number,
               payload->'pool'->>'protocol' AS protocol,
               coalesce(payload->'pool'->>'denom_symbol', payload->'pool'->>'currency') AS denom_symbol,
               payload->'pool'->>'denom_reserve' AS denom_reserve,
               payload->'pool'->>'token_reserve' AS token_reserve,
               payload->'pool'->>'price' AS price,
               (payload->'pool'->>'can_buy')::boolean AS can_buy,
               (payload->'pool'->>'can_sell')::boolean AS can_sell,
               (payload->'pool'->>'is_scam')::boolean AS is_scam
        FROM alpha_trading.strategy_observations
        WHERE run_id = $1
          AND event_source = 'pool_update'
          AND block_number = $2
          AND lower((payload->'pool'->>'token_address') || ':' || (payload->'pool'->>'pool_address')) = lower($3)
        ORDER BY first_seen_at DESC
        LIMIT 1
        "#,
    )
    .bind(replay_run_id)
    .bind(block_number)
    .bind(pool_id)
    .fetch_optional(pool)
    .await
    .wrap_err("failed to load pool observation")?;

    row.map(|row| observation_from_row(&row)).transpose()
}

fn observation_from_row(row: &sqlx::postgres::PgRow) -> Result<PoolObservation> {
    Ok(PoolObservation {
        block_number: row.try_get("block_number")?,
        protocol: row.try_get("protocol")?,
        denom_symbol: row.try_get("denom_symbol")?,
        denom_reserve: row.try_get("denom_reserve")?,
        token_reserve: row.try_get("token_reserve")?,
        price: row.try_get("price")?,
        can_buy: row.try_get("can_buy")?,
        can_sell: row.try_get("can_sell")?,
        is_scam: row.try_get("is_scam")?,
    })
}

async fn load_trajectory(
    pool: &PgPool,
    position: &PositionRecord,
    replay_run_id: &str,
    samples: i64,
) -> Result<Vec<TrajectoryPoint>> {
    let rows = sqlx::query(
        r#"
        WITH ranked AS (
            SELECT ps.*,
                   row_number() OVER (ORDER BY ps.block_number ASC, ps.id ASC) AS rn_asc,
                   row_number() OVER (ORDER BY ps.block_number DESC, ps.id DESC) AS rn_desc
            FROM alpha_trading.position_snapshots ps
            WHERE ps.run_id = $1 AND ps.position_id = $2
        )
        SELECT
            ranked.block_number,
            ranked.current_value_eth,
            ranked.unrealized_profit_eth,
            ranked.roi,
            so.payload->'pool'->>'denom_reserve' AS denom_reserve,
            so.payload->'pool'->>'token_reserve' AS token_reserve,
            so.payload->'pool'->>'price' AS spot_price,
            (so.payload->'pool'->>'can_buy')::boolean AS can_buy,
            (so.payload->'pool'->>'can_sell')::boolean AS can_sell
        FROM ranked
        LEFT JOIN alpha_trading.strategy_observations so
          ON so.run_id = $3
         AND so.event_source = 'pool_update'
         AND so.block_number = ranked.block_number
         AND lower((so.payload->'pool'->>'token_address') || ':' || (so.payload->'pool'->>'pool_address')) = lower($4)
        WHERE ranked.rn_asc <= $5 OR ranked.rn_desc <= $5
        ORDER BY ranked.block_number ASC, ranked.id ASC
        "#,
    )
    .bind(&position.run_id)
    .bind(&position.position_id)
    .bind(replay_run_id)
    .bind(&position.pool_address)
    .bind(samples)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load position trajectory")?;

    rows.into_iter()
        .map(|row| {
            Ok(TrajectoryPoint {
                block_number: row.try_get("block_number")?,
                current_value_eth: row.try_get("current_value_eth")?,
                unrealized_profit_eth: row.try_get("unrealized_profit_eth")?,
                roi: row.try_get("roi")?,
                denom_reserve: row.try_get("denom_reserve")?,
                token_reserve: row.try_get("token_reserve")?,
                spot_price: row.try_get("spot_price")?,
                can_buy: row.try_get("can_buy")?,
                can_sell: row.try_get("can_sell")?,
            })
        })
        .collect()
}
