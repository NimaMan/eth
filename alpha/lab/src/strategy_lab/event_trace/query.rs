use eyre::{Context, Result};
use sqlx::{PgPool, Row};

use super::model::{
    DecisionRow, RiskEventRow, SnapshotRow, TradeEventRow, TradeRecord, TradeTiming,
};

pub async fn load_trade(
    pool: &PgPool,
    result_set_id: &str,
    strategy_name: &str,
    trade_id: &str,
    run_id: Option<&str>,
) -> Result<TradeRecord> {
    let row = sqlx::query(
        r#"
        SELECT trade_id, token_address, pool_address, state, entry_block,
               exit_block, latest_snapshot_block, entry_cost_eth,
               exit_value_eth, current_value_eth, gas_cost_eth,
               realized_pnl_eth, unrealized_pnl_eth, total_pnl_eth, roi
        FROM alpha_trading.trades
        WHERE result_set_id = $1
          AND strategy_name = $2
          AND trade_id = $3
          AND ($4::text IS NULL OR run_id = $4)
        "#,
    )
    .bind(result_set_id)
    .bind(strategy_name)
    .bind(trade_id)
    .bind(run_id)
    .fetch_optional(pool)
    .await
    .wrap_err("failed to load trade")?
    .ok_or_else(|| eyre::eyre!("trade not found: {result_set_id} {strategy_name} {trade_id}"))?;

    trade_from_row(&row)
}

pub async fn load_losing_trade_ids(
    pool: &PgPool,
    result_set_id: &str,
    strategy_name: &str,
    run_id: Option<&str>,
) -> Result<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT trade_id
        FROM alpha_trading.trades
        WHERE result_set_id = $1
          AND strategy_name = $2
          AND ($3::text IS NULL OR run_id = $3)
          AND COALESCE(NULLIF(total_pnl_eth, '')::numeric, 0) < 0
        ORDER BY COALESCE(NULLIF(total_pnl_eth, '')::numeric, 0) ASC
        "#,
    )
    .bind(result_set_id)
    .bind(strategy_name)
    .bind(run_id)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load losing trade ids")?;

    rows.into_iter()
        .map(|row| row.try_get("trade_id").wrap_err("missing trade_id"))
        .collect()
}

pub async fn load_trade_run_id(pool: &PgPool, trade_id: &str) -> Result<String> {
    let row = sqlx::query("SELECT run_id FROM alpha_trading.trades WHERE trade_id = $1")
        .bind(trade_id)
        .fetch_optional(pool)
        .await
        .wrap_err("failed to load trade run id")?
        .ok_or_else(|| eyre::eyre!("trade_id not found: {trade_id}"))?;
    row.try_get("run_id").wrap_err("missing run_id")
}

pub async fn load_trade_events(pool: &PgPool, trade_id: &str) -> Result<Vec<TradeEventRow>> {
    let rows = sqlx::query(
        r#"
        SELECT event_type, order_side, status, order_id, block_number,
               filled_amount_raw, filled_amount_decimals, gas_cost_eth, error
        FROM alpha_trading.trade_events
        WHERE trade_id = $1
        ORDER BY COALESCE(block_number, 9223372036854775807), id
        "#,
    )
    .bind(trade_id)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load trade events")?;

    rows.into_iter()
        .map(|row| {
            Ok(TradeEventRow {
                event_type: row.try_get("event_type")?,
                order_side: row.try_get("order_side")?,
                status: row.try_get("status")?,
                order_id: row.try_get("order_id")?,
                block_number: row.try_get("block_number")?,
                filled_amount_raw: row.try_get("filled_amount_raw")?,
                filled_amount_decimals: row.try_get("filled_amount_decimals")?,
                gas_cost_eth: row.try_get("gas_cost_eth")?,
                error: row.try_get("error")?,
            })
        })
        .collect()
}

pub async fn load_risk_events(
    pool: &PgPool,
    run_id: &str,
    token_address: &str,
    pool_address: &str,
) -> Result<Vec<RiskEventRow>> {
    let rows = sqlx::query(
        r#"
        SELECT kind, severity, observed_block, message
        FROM alpha_trading.risk_events
        WHERE run_id = $1
          AND lower(token_address) = lower($2)
          AND lower(COALESCE(pool_address, '')) = lower($3)
        ORDER BY COALESCE(observed_block, 9223372036854775807), id
        "#,
    )
    .bind(run_id)
    .bind(token_address)
    .bind(pool_address)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load risk events")?;

    rows.into_iter()
        .map(|row| {
            Ok(RiskEventRow {
                kind: row.try_get("kind")?,
                severity: row.try_get("severity")?,
                observed_block: row.try_get("observed_block")?,
                message: row.try_get("message")?,
            })
        })
        .collect()
}

pub async fn load_decisions(
    pool: &PgPool,
    run_id: &str,
    strategy_name: &str,
    token_address: &str,
    pool_address: &str,
) -> Result<Vec<DecisionRow>> {
    let rows = sqlx::query(
        r#"
        SELECT event_source, event_key, block_number, action, reason, order_side
        FROM alpha_trading.strategy_decisions
        WHERE run_id = $1
          AND strategy_name = $2
          AND lower(COALESCE(token_address, '')) = lower($3)
          AND lower(COALESCE(pool_address, '')) = lower($4)
        ORDER BY COALESCE(block_number, 9223372036854775807), id
        "#,
    )
    .bind(run_id)
    .bind(strategy_name)
    .bind(token_address)
    .bind(pool_address)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load strategy decisions")?;

    rows.into_iter()
        .map(|row| {
            Ok(DecisionRow {
                event_source: row.try_get("event_source")?,
                event_key: row.try_get("event_key")?,
                block_number: row.try_get("block_number")?,
                action: row.try_get("action")?,
                reason: row.try_get("reason")?,
                order_side: row.try_get("order_side")?,
            })
        })
        .collect()
}

pub async fn load_snapshots(pool: &PgPool, trade_id: &str) -> Result<Vec<SnapshotRow>> {
    let rows = sqlx::query(
        r#"
        SELECT block_number, observed_block_number, valuation_block_number, state,
               current_value_eth, realized_pnl_eth, unrealized_pnl_eth,
               total_pnl_eth, roi, pool_price_to_initial_price_ratio,
               pool_liquidity_denom
        FROM alpha_trading.trade_snapshots
        WHERE trade_id = $1
        ORDER BY block_number, id
        "#,
    )
    .bind(trade_id)
    .fetch_all(pool)
    .await
    .wrap_err("failed to load trade snapshots")?;

    rows.into_iter()
        .map(|row| {
            Ok(SnapshotRow {
                block_number: row.try_get("block_number")?,
                observed_block_number: row.try_get("observed_block_number")?,
                valuation_block_number: row.try_get("valuation_block_number")?,
                state: row.try_get("state")?,
                current_value_eth: row.try_get("current_value_eth")?,
                realized_pnl_eth: row.try_get("realized_pnl_eth")?,
                unrealized_pnl_eth: row.try_get("unrealized_pnl_eth")?,
                total_pnl_eth: row.try_get("total_pnl_eth")?,
                roi: row.try_get("roi")?,
                pool_price_to_initial_price_ratio: row
                    .try_get("pool_price_to_initial_price_ratio")?,
                pool_liquidity_denom: row.try_get("pool_liquidity_denom")?,
            })
        })
        .collect()
}

pub fn timing_from_rows(
    trade_events: &[TradeEventRow],
    risk_events: &[RiskEventRow],
) -> TradeTiming {
    TradeTiming {
        buy_submitted_block: first_trade_event_block(trade_events, "buy_submitted"),
        buy_confirmed_block: first_trade_event_block(trade_events, "buy_confirmed"),
        sell_submitted_block: first_trade_event_block(trade_events, "sell_submitted"),
        sell_confirmed_block: first_trade_event_block(trade_events, "sell_confirmed"),
        first_lp_approval_block: first_risk_block(risk_events, "lp_approval"),
        first_liquidity_removal_block: first_risk_block(risk_events, "liquidity_removal"),
        lp_approval_count: risk_events
            .iter()
            .filter(|event| event.kind == "lp_approval")
            .count() as i64,
        liquidity_removal_count: risk_events
            .iter()
            .filter(|event| event.kind == "liquidity_removal")
            .count() as i64,
    }
}

fn first_trade_event_block(events: &[TradeEventRow], event_type: &str) -> Option<i64> {
    events
        .iter()
        .filter(|event| event.event_type == event_type)
        .filter_map(|event| event.block_number)
        .min()
}

fn first_risk_block(events: &[RiskEventRow], kind: &str) -> Option<i64> {
    events
        .iter()
        .filter(|event| event.kind == kind)
        .filter_map(|event| event.observed_block)
        .min()
}

fn trade_from_row(row: &sqlx::postgres::PgRow) -> Result<TradeRecord> {
    Ok(TradeRecord {
        trade_id: row.try_get("trade_id")?,
        token_address: row.try_get("token_address")?,
        pool_address: row.try_get("pool_address")?,
        state: row.try_get("state")?,
        entry_block: row.try_get("entry_block")?,
        exit_block: row.try_get("exit_block")?,
        latest_snapshot_block: row.try_get("latest_snapshot_block")?,
        entry_cost_eth: row.try_get("entry_cost_eth")?,
        exit_value_eth: row.try_get("exit_value_eth")?,
        current_value_eth: row.try_get("current_value_eth")?,
        gas_cost_eth: row.try_get("gas_cost_eth")?,
        realized_pnl_eth: row.try_get("realized_pnl_eth")?,
        unrealized_pnl_eth: row.try_get("unrealized_pnl_eth")?,
        total_pnl_eth: row.try_get("total_pnl_eth")?,
        roi: row.try_get("roi")?,
    })
}
