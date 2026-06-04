use serde_json::{json, Value};
use sqlx::Row;

use super::rows::{
    eth_address_aggregate_json, eth_address_type_and_flags, eth_trader_label_breakdown_row,
    eth_trader_mechanism_breakdown_row, eth_trader_movement_row, eth_trader_pool_position_row,
    eth_trader_row, eth_trader_summary_row,
};
use super::run::token_pnl_run_json;
use super::sql::{
    eth_trader_filtered_summary_sql, eth_trader_label_breakdown_sql,
    eth_trader_mechanism_breakdown_sql, eth_trader_position_detail_sql, eth_trader_positions_sql,
    eth_trader_profile_summary_sql, eth_trader_recent_movements_sql, eth_trader_rows_sql,
    eth_trader_trade_movements_sql,
};
use super::{EthTraderListParams, TokenPnlReader};
use crate::Result;

// Address PnL endpoints compose one selected run with address aggregates,
// position-level details, and retained movement rows for route JSON payloads.
impl TokenPnlReader {
    pub async fn eth_traders(&self, params: EthTraderListParams) -> Result<Option<Value>> {
        let Some(run) = self.latest_token_pnl_run().await? else {
            return Ok(None);
        };
        let run_id: String = run.try_get("run_id")?;
        let params = NormalizedEthTraderListParams::from(params);

        let summary_sql = eth_trader_filtered_summary_sql();
        let rows_sql = eth_trader_rows_sql(trader_sort_expression(&params.sort));

        let summary = bind_eth_trader_filters(sqlx::query(&summary_sql), &run_id, &params)
            .fetch_one(&self.pool)
            .await?;
        let rows = bind_eth_trader_filters(sqlx::query(&rows_sql), &run_id, &params)
            .bind(params.page_size)
            .bind(params.offset())
            .fetch_all(&self.pool)
            .await?;
        let total_count: i64 = summary.try_get("address_count")?;

        Ok(Some(json!({
            "run": token_pnl_run_json(&run, &run_id)?,
            "summary": eth_trader_summary_row(summary)?,
            "rows": rows
                .into_iter()
                .map(eth_trader_row)
                .collect::<Result<Vec<_>>>()?,
            "page": params.page,
            "pageSize": params.page_size,
            "totalCount": total_count,
            "totalPages": total_pages(total_count, params.page_size),
            "filters": params.filters_json(),
        })))
    }

    pub async fn eth_trader_profile(&self, address: &str) -> Result<Option<Value>> {
        let Some(run) = self.latest_token_pnl_run().await? else {
            return Ok(None);
        };
        let run_id: String = run.try_get("run_id")?;

        let summary_sql = eth_trader_profile_summary_sql();
        let Some(summary) = sqlx::query(&summary_sql)
            .bind(&run_id)
            .bind(address)
            .fetch_optional(&self.pool)
            .await?
        else {
            return Ok(None);
        };

        let top_positions_sql = eth_trader_positions_sql(
            "abs_denom_cashflow DESC, COALESCE(a.latest_block, 0) DESC, s.pool_id",
        );
        let recent_positions_sql = eth_trader_positions_sql(
            "COALESCE(a.latest_block, 0) DESC, abs_denom_cashflow DESC, s.pool_id",
        );
        let mechanism_breakdown_sql = eth_trader_mechanism_breakdown_sql();
        let label_breakdown_sql = eth_trader_label_breakdown_sql();
        let recent_movements_sql = eth_trader_recent_movements_sql();

        let top_positions = sqlx::query(&top_positions_sql)
            .bind(&run_id)
            .bind(address)
            .bind(25_i64)
            .fetch_all(&self.pool)
            .await?;
        let recent_positions = sqlx::query(&recent_positions_sql)
            .bind(&run_id)
            .bind(address)
            .bind(25_i64)
            .fetch_all(&self.pool)
            .await?;
        let mechanism_breakdowns = sqlx::query(&mechanism_breakdown_sql)
            .bind(&run_id)
            .bind(address)
            .bind(20_i64)
            .fetch_all(&self.pool)
            .await?;
        let label_breakdowns = sqlx::query(&label_breakdown_sql)
            .bind(&run_id)
            .bind(address)
            .bind(30_i64)
            .fetch_all(&self.pool)
            .await?;
        let recent_movements = sqlx::query(&recent_movements_sql)
            .bind(&run_id)
            .bind(address)
            .bind(100_i64)
            .fetch_all(&self.pool)
            .await?;

        let aggregate_pnl = eth_address_aggregate_json(&summary)?;
        let (address_type, validity_flags, admissibility) = eth_address_type_and_flags(&summary)?;
        let activity = json!({
            "first_block": summary.try_get::<Option<i64>, _>("first_block")?,
            "latest_block": summary.try_get::<Option<i64>, _>("latest_block")?,
        });
        let summary_json = eth_trader_row(summary)?;

        Ok(Some(json!({
            "run": token_pnl_run_json(&run, &run_id)?,
            "summary": summary_json,
            "aggregatePnl": aggregate_pnl,
            "addressType": address_type,
            "validityFlags": validity_flags,
            "admissibility": admissibility,
            "activity": activity,
            "topPoolPositions": top_positions
                .into_iter()
                .map(eth_trader_pool_position_row)
                .collect::<Result<Vec<_>>>()?,
            "recentPoolPositions": recent_positions
                .into_iter()
                .map(eth_trader_pool_position_row)
                .collect::<Result<Vec<_>>>()?,
            "mechanismBreakdowns": mechanism_breakdowns
                .into_iter()
                .map(eth_trader_mechanism_breakdown_row)
                .collect::<Result<Vec<_>>>()?,
            "labelBreakdowns": label_breakdowns
                .into_iter()
                .map(eth_trader_label_breakdown_row)
                .collect::<Result<Vec<_>>>()?,
            "recentMovements": recent_movements
                .into_iter()
                .map(eth_trader_movement_row)
                .collect::<Result<Vec<_>>>()?,
        })))
    }

    pub async fn eth_trader_trade(&self, address: &str, pool_id: &str) -> Result<Option<Value>> {
        let Some(run) = self.latest_token_pnl_run().await? else {
            return Ok(None);
        };
        let run_id: String = run.try_get("run_id")?;

        let summary_sql = eth_trader_profile_summary_sql();
        let Some(summary) = sqlx::query(&summary_sql)
            .bind(&run_id)
            .bind(address)
            .fetch_optional(&self.pool)
            .await?
        else {
            return Ok(None);
        };

        let position_sql = eth_trader_position_detail_sql();
        let Some(position) = sqlx::query(&position_sql)
            .bind(&run_id)
            .bind(address)
            .bind(pool_id)
            .fetch_optional(&self.pool)
            .await?
        else {
            return Ok(None);
        };

        let movement_limit = 500_i64;
        let movements_sql = eth_trader_trade_movements_sql();
        let movements = sqlx::query(&movements_sql)
            .bind(&run_id)
            .bind(address)
            .bind(pool_id)
            .bind(movement_limit)
            .fetch_all(&self.pool)
            .await?;
        let position_value = eth_trader_pool_position_row(position)?;
        let accounting_fields_populated = position_value
            .get("position_status")
            .and_then(Value::as_str)
            .map(|status| status != "unknown")
            .unwrap_or(false);

        Ok(Some(json!({
            "run": token_pnl_run_json(&run, &run_id)?,
            "trader": eth_trader_row(summary)?,
            "position": position_value,
            "movements": movements
                .into_iter()
                .map(eth_trader_movement_row)
                .collect::<Result<Vec<_>>>()?,
            "movementLimit": movement_limit,
            "source": "trade-detail",
            "apiStatus": {
                "source": "token_pnl.pool_address_pnl",
                "accounting_fields_populated": accounting_fields_populated,
                "notes": Vec::<String>::new(),
            },
        })))
    }
}

#[derive(Debug, Clone)]
struct NormalizedEthTraderListParams {
    mode: String,
    sort: String,
    min_scam_ratio: Option<f64>,
    min_trades: Option<i64>,
    page: i64,
    page_size: i64,
    mechanism: Option<String>,
    label: Option<String>,
    role: Option<String>,
    defaulted_min_scam_ratio: bool,
    defaulted_min_trades: bool,
}

impl From<EthTraderListParams> for NormalizedEthTraderListParams {
    fn from(params: EthTraderListParams) -> Self {
        let mode = normalize_trader_mode(params.mode.as_deref());
        let sort = normalize_trader_sort(params.sort.as_deref());
        let user_min_scam_ratio = params
            .min_scam_ratio
            .filter(|value| value.is_finite())
            .map(|value| value.clamp(0.0, 1.0));
        let user_min_trades = params.min_trades.filter(|value| *value >= 0);
        let defaulted_min_scam_ratio = mode == "inflation" && user_min_scam_ratio.is_none();
        let defaulted_min_trades = mode == "inflation" && user_min_trades.is_none();

        Self {
            mode,
            sort,
            min_scam_ratio: user_min_scam_ratio.or(if defaulted_min_scam_ratio {
                Some(0.75)
            } else {
                None
            }),
            min_trades: user_min_trades.or(if defaulted_min_trades { Some(50) } else { None }),
            page: params.page.unwrap_or(1).max(1),
            page_size: params.page_size.unwrap_or(50).clamp(1, 250),
            mechanism: clean_filter(params.mechanism),
            label: clean_filter(params.label),
            role: clean_filter(params.role).map(|role| role.to_ascii_lowercase().replace('-', "_")),
            defaulted_min_scam_ratio,
            defaulted_min_trades,
        }
    }
}

impl NormalizedEthTraderListParams {
    fn offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }

    fn filters_json(&self) -> Value {
        json!({
            "mode": &self.mode,
            "sort": &self.sort,
            "minScamRatio": self.min_scam_ratio,
            "minTrades": self.min_trades,
            "page": self.page,
            "pageSize": self.page_size,
            "mechanism": self.mechanism.as_deref(),
            "label": self.label.as_deref(),
            "role": self.role.as_deref(),
            "defaultsApplied": {
                "minScamRatio": self.defaulted_min_scam_ratio,
                "minTrades": self.defaulted_min_trades,
            },
        })
    }
}

fn clean_filter(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_trader_mode(value: Option<&str>) -> String {
    match value
        .unwrap_or("inflation")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "leaderboard" => "leaderboard",
        "custody" => "custody",
        "creators" | "creator" => "creators",
        "all" => "all",
        _ => "inflation",
    }
    .to_string()
}

fn normalize_trader_sort(value: Option<&str>) -> String {
    match value
        .unwrap_or("inflation_score")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "scam_ratio" => "scam_ratio",
        "trades" | "trade_count" => "trades",
        "scam_eth" | "scam_abs_denom_flow" => "scam_eth",
        "scam_tokens" | "scam_token_count" => "scam_tokens",
        _ => "inflation_score",
    }
    .to_string()
}

fn trader_sort_expression(sort: &str) -> &'static str {
    match sort {
        "scam_ratio" => {
            "scam_ratio DESC NULLS LAST, scam_pool_position_count DESC, trade_count DESC, total_abs_denom_flow DESC"
        }
        "trades" => {
            "trade_count DESC, scam_ratio DESC NULLS LAST, scam_pool_position_count DESC, total_abs_denom_flow DESC"
        }
        "scam_eth" => {
            "scam_abs_denom_flow DESC NULLS LAST, scam_ratio DESC NULLS LAST, trade_count DESC"
        }
        "scam_tokens" => {
            "scam_token_count DESC, scam_ratio DESC NULLS LAST, trade_count DESC, scam_abs_denom_flow DESC NULLS LAST"
        }
        _ => {
            "inflation_score DESC NULLS LAST, scam_ratio DESC NULLS LAST, trade_count DESC, scam_abs_denom_flow DESC NULLS LAST"
        }
    }
}

fn bind_eth_trader_filters<'q>(
    query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    run_id: &'q str,
    params: &'q NormalizedEthTraderListParams,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    query
        .bind(run_id)
        .bind(&params.mode)
        .bind(params.min_scam_ratio)
        .bind(params.min_trades)
        .bind(params.mechanism.as_deref())
        .bind(params.label.as_deref())
        .bind(params.role.as_deref())
}

fn total_pages(total_count: i64, page_size: i64) -> i64 {
    if total_count <= 0 {
        0
    } else {
        (total_count + page_size - 1) / page_size
    }
}
