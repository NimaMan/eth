use eyre::{Context, Result};
use sqlx::{PgPool, Row};

use super::model::{
    ExecutionReportRecord, PoolObservation, PositionRecord, PositionSelector, SnapshotRecord,
    TrajectoryPoint,
};

pub(super) async fn load_position(
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

pub(super) async fn load_entry_report(
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

pub(super) async fn load_latest_snapshot(
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

pub(super) async fn load_pool_observation(
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

pub(super) async fn load_trajectory(
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
