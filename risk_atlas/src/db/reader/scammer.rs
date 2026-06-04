use eth_pnl::reader::run::{latest_token_pnl_run, token_pnl_run_json};
use eth_pnl::reader::sql::ADDRESS_PNL_AGG_CTE;
use eyre::Result;
use serde_json::{json, Value};
use sqlx::Row;

use super::RiskAtlasReader;

impl RiskAtlasReader {
    pub async fn scammer_address_distribution(&self, limit: i64) -> Result<Option<Value>> {
        let limit = limit.clamp(1, 500);
        let Some(run) = latest_token_pnl_run(&self.pool).await? else {
            return Ok(None);
        };
        let run_id: String = run.try_get("run_id")?;

        let summary_sql = scammer_address_summary_sql();
        let buckets_sql = scammer_address_buckets_sql();
        let top_sql = scammer_address_top_sql();

        let summary = sqlx::query(&summary_sql)
            .bind(&run_id)
            .fetch_one(&self.pool)
            .await?;
        let buckets = sqlx::query(&buckets_sql)
            .bind(&run_id)
            .fetch_all(&self.pool)
            .await?;
        let top_addresses = sqlx::query(&top_sql)
            .bind(&run_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        Ok(Some(json!({
            "run": token_pnl_run_json(&run, &run_id)?,
            "summary": {
                "address_count": summary.try_get::<i64, _>("address_count")?,
                "pool_position_count": summary.try_get::<i64, _>("pool_position_count")?,
                "scam_pool_position_count": summary.try_get::<i64, _>("scam_pool_position_count")?,
                "scam_address_count": summary.try_get::<i64, _>("scam_address_count")?,
                "all_scam_address_count": summary.try_get::<i64, _>("all_scam_address_count")?,
                "high_scam_ratio_address_count": summary.try_get::<i64, _>("high_scam_ratio_address_count")?,
                "trade_count": summary.try_get::<i64, _>("trade_count")?,
                "exact_trade_count": summary.try_get::<i64, _>("exact_trade_count")?,
                "movement_count": summary.try_get::<i64, _>("movement_count")?,
                "total_abs_denom_flow": summary.try_get::<f64, _>("total_abs_denom_flow")?,
                "scam_abs_denom_flow": summary.try_get::<f64, _>("scam_abs_denom_flow")?,
                "latest_block": summary.try_get::<Option<i64>, _>("latest_block")?,
                "movement_rows": summary.try_get::<i64, _>("movement_rows")?,
            },
            "buckets": buckets
                .into_iter()
                .map(|row| {
                    Ok(json!({
                        "bucket": row.try_get::<String, _>("bucket")?,
                        "count": row.try_get::<i64, _>("count")?,
                        "share": row.try_get::<Option<f64>, _>("share")?,
                        "avg_trade_count": row.try_get::<Option<f64>, _>("avg_trade_count")?,
                        "avg_scam_ratio": row.try_get::<Option<f64>, _>("avg_scam_ratio")?,
                        "sort_order": row.try_get::<i32, _>("sort_order")?,
                    }))
                })
                .collect::<Result<Vec<_>>>()?,
            "top_addresses": top_addresses
                .into_iter()
                .map(address_distribution_row)
                .collect::<Result<Vec<_>>>()?,
        })))
    }
}

const SCAMMER_ADDRESS_AGG_CTE: &str = ADDRESS_PNL_AGG_CTE;

fn scammer_address_summary_sql() -> String {
    format!(
        "{SCAMMER_ADDRESS_AGG_CTE}
        , summary AS (
        SELECT
            COUNT(*)::bigint AS address_count,
            COALESCE(SUM(pool_position_count), 0)::bigint AS pool_position_count,
            COALESCE(SUM(scam_pool_position_count), 0)::bigint AS scam_pool_position_count,
            COUNT(*) FILTER (WHERE scam_pool_position_count > 0)::bigint AS scam_address_count,
            COUNT(*) FILTER (WHERE scam_ratio = 1.0)::bigint AS all_scam_address_count,
            COUNT(*) FILTER (WHERE scam_ratio >= 0.75 AND scam_pool_position_count > 0)::bigint AS high_scam_ratio_address_count,
            COALESCE(SUM(trade_count), 0)::bigint AS trade_count,
            COALESCE(SUM(exact_trade_count), 0)::bigint AS exact_trade_count,
            COALESCE(SUM(movement_count), 0)::bigint AS movement_count,
            COALESCE(SUM(total_abs_denom_flow), 0.0)::double precision AS total_abs_denom_flow,
            COALESCE(SUM(scam_abs_denom_flow), 0.0)::double precision AS scam_abs_denom_flow,
            MAX(latest_block) AS latest_block
        FROM address_agg
        )
        SELECT summary.*, mt.movement_rows
        FROM summary
        CROSS JOIN movement_total mt"
    )
}

fn scammer_address_buckets_sql() -> String {
    format!(
        "{SCAMMER_ADDRESS_AGG_CTE}
        SELECT
            bucket,
            COUNT(*)::bigint AS count,
            COUNT(*)::double precision / NULLIF((SELECT COUNT(*)::double precision FROM address_agg), 0.0) AS share,
            AVG(trade_count::double precision) AS avg_trade_count,
            AVG(scam_ratio) AS avg_scam_ratio,
            sort_order
        FROM (
            SELECT
                *,
                CASE
                    WHEN scam_ratio = 0.0 THEN '0% scam pools'
                    WHEN scam_ratio < 0.25 THEN '<25% scam pools'
                    WHEN scam_ratio < 0.50 THEN '25-50% scam pools'
                    WHEN scam_ratio < 0.75 THEN '50-75% scam pools'
                    WHEN scam_ratio < 1.0 THEN '75-99% scam pools'
                    ELSE '100% scam pools'
                END AS bucket,
                CASE
                    WHEN scam_ratio = 0.0 THEN 0
                    WHEN scam_ratio < 0.25 THEN 1
                    WHEN scam_ratio < 0.50 THEN 2
                    WHEN scam_ratio < 0.75 THEN 3
                    WHEN scam_ratio < 1.0 THEN 4
                    ELSE 5
                END AS sort_order
            FROM address_agg
        ) bucketed
        GROUP BY bucket, sort_order
        ORDER BY sort_order"
    )
}

fn scammer_address_top_sql() -> String {
    format!(
        "{SCAMMER_ADDRESS_AGG_CTE}
        SELECT
            aa.*,
            COALESCE(la.pool_labels, ARRAY[]::text[]) AS pool_labels,
            ARRAY(
                SELECT DISTINCT role
                FROM unnest(
                    COALESCE(ra.actor_roles, ARRAY[]::text[])
                    || ARRAY_REMOVE(ARRAY[
                        CASE WHEN aa.token_creator_position_count > 0 THEN 'token_creator'::text END,
                        CASE WHEN aa.pool_creator_position_count > 0 THEN 'pool_creator'::text END
                    ], NULL)
                ) AS role(role)
                ORDER BY role
            ) AS role_flags,
            mt.movement_rows
        FROM address_agg aa
        LEFT JOIN label_agg la USING (address)
        LEFT JOIN role_agg ra USING (address)
        CROSS JOIN movement_total mt
        ORDER BY
            aa.scam_ratio DESC NULLS LAST,
            aa.scam_pool_position_count DESC,
            aa.trade_count DESC,
            aa.total_abs_denom_flow DESC
        LIMIT $2"
    )
}

fn address_distribution_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    Ok(json!({
        "address": row.try_get::<String, _>("address")?,
        "pool_position_count": row.try_get::<i64, _>("pool_position_count")?,
        "token_count": row.try_get::<i64, _>("token_count")?,
        "scam_pool_position_count": row.try_get::<i64, _>("scam_pool_position_count")?,
        "scam_token_count": row.try_get::<i64, _>("scam_token_count")?,
        "trade_count": row.try_get::<i64, _>("trade_count")?,
        "exact_trade_count": row.try_get::<i64, _>("exact_trade_count")?,
        "movement_count": row.try_get::<i64, _>("movement_count")?,
        "first_block": row.try_get::<Option<i64>, _>("first_block")?,
        "latest_block": row.try_get::<Option<i64>, _>("latest_block")?,
        "denom_in": row.try_get::<Option<f64>, _>("denom_in")?,
        "denom_out": row.try_get::<Option<f64>, _>("denom_out")?,
        "net_denom_cashflow": row.try_get::<Option<f64>, _>("net_denom_cashflow")?,
        "total_abs_denom_flow": row.try_get::<Option<f64>, _>("total_abs_denom_flow")?,
        "scam_abs_denom_flow": row.try_get::<Option<f64>, _>("scam_abs_denom_flow")?,
        "scam_ratio": row.try_get::<Option<f64>, _>("scam_ratio")?,
        "scam_token_ratio": row.try_get::<Option<f64>, _>("scam_token_ratio")?,
        "scam_mechanisms": row.try_get::<Option<Vec<String>>, _>("scam_mechanisms")?.unwrap_or_default(),
        "scam_labels": row.try_get::<Option<Vec<String>>, _>("scam_labels")?.unwrap_or_default(),
        "lifecycles": row.try_get::<Option<Vec<String>>, _>("lifecycles")?.unwrap_or_default(),
        "pool_labels": row.try_get::<Vec<String>, _>("pool_labels")?,
        "role_flags": row.try_get::<Vec<String>, _>("role_flags")?,
        "token_creator_position_count": row.try_get::<i64, _>("token_creator_position_count")?,
        "pool_creator_position_count": row.try_get::<i64, _>("pool_creator_position_count")?,
        "movement_rows_available": row.try_get::<i64, _>("movement_rows")? > 0,
    }))
}
