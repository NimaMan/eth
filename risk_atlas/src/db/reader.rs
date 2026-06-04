use eyre::Result;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use crate::api::RiskAtlasPageView;
use crate::atlas::{build_page_story, default_story_sections};

use super::schema::{
    ActiveTargetSummary, DecisionQuestion, DistributionBucket, EventEvidenceRow,
    ModelReadinessItem, NumericStat, ReviewExample, RiskAtlasRun,
};

#[derive(Clone)]
pub struct RiskAtlasReader {
    pool: PgPool,
}

#[derive(Debug, Clone, Default)]
pub struct EthTraderListParams {
    pub mode: Option<String>,
    pub sort: Option<String>,
    pub min_scam_ratio: Option<f64>,
    pub min_trades: Option<i64>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub mechanism: Option<String>,
    pub label: Option<String>,
    pub role: Option<String>,
}

impl RiskAtlasReader {
    pub async fn connect(database_url: &str) -> Result<Self> {
        Ok(Self {
            pool: PgPool::connect(database_url).await?,
        })
    }

    pub fn connect_lazy(database_url: &str) -> Result<Self> {
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(5)
                .connect_lazy(database_url)?,
        })
    }

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn latest_page_view(&self) -> Result<Option<RiskAtlasPageView>> {
        let row = sqlx::query(
            r#"
            SELECT run_id
            FROM risk_atlas_runs
            ORDER BY generated_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };
        let run_id: String = row.try_get("run_id")?;
        self.page_view(&run_id).await
    }

    pub async fn runs(&self, limit: i64) -> Result<Vec<RiskAtlasRun>> {
        let limit = limit.clamp(1, 500);
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM risk_atlas_runs
            ORDER BY generated_at DESC, run_id DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_run).collect()
    }

    pub async fn page_view(&self, run_id: &str) -> Result<Option<RiskAtlasPageView>> {
        let Some(run) = self.run(run_id).await? else {
            return Ok(None);
        };

        let distributions = self.distributions(run_id).await?;
        let numeric_stats = self.numeric_stats(run_id).await?;
        let active_targets = self.active_targets(run_id).await?;
        let page_story = build_page_story(&run, &distributions, &numeric_stats, &active_targets);

        Ok(Some(RiskAtlasPageView {
            run,
            page_story,
            sections: default_story_sections(),
            distributions,
            numeric_stats,
            active_targets,
            event_evidence: self.event_evidence(run_id).await?,
            decision_questions: self.decision_questions(run_id).await?,
            review_examples: self.review_examples(run_id).await?,
            model_readiness: self.model_readiness(run_id).await?,
        }))
    }

    pub async fn scammer_address_distribution(&self, limit: i64) -> Result<Option<Value>> {
        let limit = limit.clamp(1, 500);
        let Some(run) = self.latest_token_pnl_run().await? else {
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
            "run": {
                "run_id": run_id,
                "mode": run.try_get::<String, _>("mode")?,
                "algorithm_version": run.try_get::<String, _>("algorithm_version")?,
                "start_block": run.try_get::<Option<i64>, _>("start_block")?,
                "end_block": run.try_get::<Option<i64>, _>("end_block")?,
                "status": run.try_get::<String, _>("status")?,
                "metadata": run.try_get::<Value, _>("metadata")?,
                "created_at": run.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")?,
                "updated_at": run.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")?,
            },
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
        let (address_type, validity_flags) = eth_address_type_and_flags(&summary)?;
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

    async fn latest_token_pnl_run(&self) -> Result<Option<sqlx::postgres::PgRow>> {
        Ok(sqlx::query(
            r#"
            SELECT run_id, mode, algorithm_version, start_block, end_block,
                   status, metadata, created_at, updated_at
            FROM token_pnl.calculation_runs
            ORDER BY (status = 'complete') DESC, updated_at DESC, run_id DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn run(&self, run_id: &str) -> Result<Option<RiskAtlasRun>> {
        let row = sqlx::query(
            r#"
            SELECT *
            FROM risk_atlas_runs
            WHERE run_id = $1
            "#,
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_run).transpose()
    }

    async fn distributions(&self, run_id: &str) -> Result<Vec<DistributionBucket>> {
        let rows = sqlx::query(
            r#"
            SELECT section, bucket, count, share, sort_order
            FROM risk_atlas_distributions
            WHERE run_id = $1
            ORDER BY section, sort_order, bucket
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_distribution).collect()
    }

    async fn numeric_stats(&self, run_id: &str) -> Result<Vec<NumericStat>> {
        let rows = sqlx::query(
            r#"
            SELECT section, metric, count, min, p25, median, p75, p90, p95, max, sort_order
            FROM risk_atlas_numeric_stats
            WHERE run_id = $1
            ORDER BY section, sort_order, metric
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_numeric_stat).collect()
    }

    async fn active_targets(&self, run_id: &str) -> Result<Vec<ActiveTargetSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT row_kind, horizon_active_observations, rows, unique_pools, positives, negatives, sort_order
            FROM risk_atlas_active_targets
            WHERE run_id = $1
            ORDER BY sort_order, horizon_active_observations NULLS FIRST, row_kind
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_active_target).collect()
    }

    async fn event_evidence(&self, run_id: &str) -> Result<Vec<EventEvidenceRow>> {
        let rows = sqlx::query(
            r#"
            SELECT token_address, pool_address, protocol, event_kind, mechanism,
                   block_number, block_timestamp, tx_hash, mempool_first_seen_ms,
                   source, sort_order
            FROM risk_atlas_event_evidence
            WHERE run_id = $1
            ORDER BY sort_order, block_number NULLS LAST, token_address, pool_address
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_event_evidence).collect()
    }

    async fn decision_questions(&self, run_id: &str) -> Result<Vec<DecisionQuestion>> {
        let rows = sqlx::query(
            r#"
            SELECT question_id, category, question, headline, answer, status,
                   denominator_label, denominator_count, payload, sort_order
            FROM risk_atlas_decision_questions
            WHERE run_id = $1
            ORDER BY sort_order, question_id
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_decision_question).collect()
    }

    async fn review_examples(&self, run_id: &str) -> Result<Vec<ReviewExample>> {
        let rows = sqlx::query(
            r#"
            SELECT queue, token_address, pool_address, protocol, symbol, mechanism, mechanism_label,
                   label_block, trading_enabled_block, age_blocks, liquidity_eth,
                   price_ratio_to_initial, evidence_summary, evidence_ref, sort_order
            FROM risk_atlas_review_examples
            WHERE run_id = $1
            ORDER BY queue, sort_order, token_address, pool_address
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_review_example).collect()
    }

    async fn model_readiness(&self, run_id: &str) -> Result<Vec<ModelReadinessItem>> {
        let rows = sqlx::query(
            r#"
            SELECT name, status, detail, sort_order
            FROM risk_atlas_model_readiness
            WHERE run_id = $1
            ORDER BY sort_order, name
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_model_readiness).collect()
    }
}

const SCAMMER_ADDRESS_AGG_CTE: &str = r#"
WITH movement_counts AS (
    SELECT run_id, pool_id, address, COUNT(DISTINCT tx_hash)::bigint AS exact_trade_count
    FROM token_pnl.pool_pnl_movements
    WHERE run_id = $1
    GROUP BY run_id, pool_id, address
),
address_rows AS (
    SELECT
        a.address,
        a.pool_id,
        s.token_address,
        s.is_scam,
        s.scam_label,
        s.scam_mechanism,
        s.lifecycle,
        s.pool_labels,
        s.token_creator_address,
        s.pool_creator_address,
        a.position_status,
        a.valuation_status,
        a.reconciliation_status,
        a.movement_rows_retained,
        a.movement_rows_backed,
        a.actor_roles,
        a.is_user_candidate,
        a.realized_pnl_denom,
        a.unrealized_value_denom,
        a.total_pnl_denom,
        a.first_block,
        a.latest_block,
        COALESCE(a.movement_count, 0)::bigint AS movement_count,
        COALESCE(m.exact_trade_count, 0)::bigint AS exact_trade_count,
        COALESCE(a.denom_cashflow::double precision, 0.0) AS denom_cashflow,
        ABS(COALESCE(a.denom_cashflow::double precision, 0.0)) AS abs_denom_cashflow,
        (a.denom_in_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_in,
        (a.denom_out_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_out,
        COALESCE(a.native_fee::double precision, 0.0) AS native_fee,
        COALESCE(a.native_priority_fee::double precision, 0.0) AS native_priority_fee,
        (lower(s.denom_address) = '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2') AS denom_is_eth
    FROM token_pnl.pool_address_pnl a
    JOIN token_pnl.pool_pnl_states s
      ON s.run_id = a.run_id AND s.pool_id = a.pool_id
    LEFT JOIN movement_counts m
      ON m.run_id = a.run_id AND m.pool_id = a.pool_id AND m.address = a.address
    WHERE a.run_id = $1
),
address_agg AS (
    SELECT
        address,
        COUNT(*)::bigint AS pool_position_count,
        COUNT(DISTINCT token_address)::bigint AS token_count,
        COUNT(*) FILTER (WHERE is_scam)::bigint AS scam_pool_position_count,
        COUNT(DISTINCT token_address) FILTER (WHERE is_scam)::bigint AS scam_token_count,
        SUM(CASE WHEN exact_trade_count > 0 THEN exact_trade_count ELSE movement_count END)::bigint AS trade_count,
        SUM(exact_trade_count)::bigint AS exact_trade_count,
        SUM(movement_count)::bigint AS movement_count,
        MIN(first_block) AS first_block,
        MAX(latest_block) AS latest_block,
        SUM(denom_in) AS denom_in,
        SUM(denom_out) AS denom_out,
        SUM(denom_cashflow) AS net_denom_cashflow,
        SUM(abs_denom_cashflow) AS total_abs_denom_flow,
        SUM(CASE WHEN is_scam THEN abs_denom_cashflow ELSE 0.0 END) AS scam_abs_denom_flow,
        (COUNT(*) FILTER (WHERE is_scam))::double precision / NULLIF(COUNT(*)::double precision, 0.0) AS scam_ratio,
        (COUNT(DISTINCT token_address) FILTER (WHERE is_scam))::double precision / NULLIF(COUNT(DISTINCT token_address)::double precision, 0.0) AS scam_token_ratio,
        ARRAY_AGG(DISTINCT scam_mechanism) FILTER (WHERE scam_mechanism IS NOT NULL) AS scam_mechanisms,
        ARRAY_AGG(DISTINCT scam_label) FILTER (WHERE scam_label IS NOT NULL) AS scam_labels,
        ARRAY_AGG(DISTINCT lifecycle) FILTER (WHERE lifecycle IS NOT NULL) AS lifecycles,
        SUM(CASE WHEN lower(address) = lower(COALESCE(token_creator_address, '')) THEN 1 ELSE 0 END)::bigint AS token_creator_position_count,
        SUM(CASE WHEN lower(address) = lower(COALESCE(pool_creator_address, '')) THEN 1 ELSE 0 END)::bigint AS pool_creator_position_count,
        SUM(realized_pnl_denom)::double precision AS realized_pnl_denom_sum,
        SUM(unrealized_value_denom)::double precision AS unrealized_pnl_denom_sum,
        SUM(total_pnl_denom)::double precision AS total_pnl_denom_sum,
        SUM(native_fee + native_priority_fee)::double precision AS gas_paid,
        COUNT(*) FILTER (WHERE total_pnl_denom > 0)::bigint AS win_count,
        COUNT(*) FILTER (WHERE total_pnl_denom < 0)::bigint AS loss_count,
        COUNT(*) FILTER (WHERE total_pnl_denom = 0 OR total_pnl_denom IS NULL)::bigint AS breakeven_count,
        SUM(movement_rows_retained)::bigint AS movement_rows_retained_sum,
        COUNT(*) FILTER (WHERE movement_rows_backed)::bigint AS movement_backed_position_count,
        BOOL_AND(denom_is_eth) AS all_denom_eth,
        BOOL_OR(is_user_candidate) AS any_user_candidate
    FROM address_rows
    GROUP BY address
),
label_agg AS (
    SELECT
        ar.address,
        ARRAY_AGG(DISTINCT label.value ORDER BY label.value) FILTER (WHERE label.value IS NOT NULL) AS pool_labels
    FROM address_rows ar
    LEFT JOIN LATERAL jsonb_array_elements_text(COALESCE(ar.pool_labels, '[]'::jsonb)) AS label(value) ON true
    GROUP BY ar.address
),
role_agg AS (
    SELECT
        ar.address,
        ARRAY_AGG(DISTINCT role.value ORDER BY role.value) FILTER (WHERE role.value IS NOT NULL) AS actor_roles
    FROM address_rows ar
    LEFT JOIN LATERAL jsonb_array_elements_text(COALESCE(ar.actor_roles, '[]'::jsonb)) AS role(value) ON true
    GROUP BY ar.address
),
movement_total AS (
    SELECT COUNT(*)::bigint AS movement_rows
    FROM token_pnl.pool_pnl_movements
    WHERE run_id = $1
)
"#;

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

fn token_pnl_run_json(row: &sqlx::postgres::PgRow, run_id: &str) -> Result<Value> {
    Ok(json!({
        "run_id": run_id,
        "mode": row.try_get::<String, _>("mode")?,
        "algorithm_version": row.try_get::<String, _>("algorithm_version")?,
        "start_block": row.try_get::<Option<i64>, _>("start_block")?,
        "end_block": row.try_get::<Option<i64>, _>("end_block")?,
        "status": row.try_get::<String, _>("status")?,
        "metadata": row.try_get::<Value, _>("metadata")?,
        "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")?,
        "updated_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")?,
    }))
}

fn eth_trader_ranked_cte() -> String {
    format!(
        "{SCAMMER_ADDRESS_AGG_CTE}
        , ranked AS (
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
                mt.movement_rows,
                (
                    COALESCE(aa.scam_ratio, 0.0)
                    * LN(1.0 + GREATEST(aa.trade_count::double precision, 0.0))
                    * LN(1.0 + GREATEST(aa.scam_token_count::double precision, 0.0))
                    * LN(1.0 + GREATEST(COALESCE(aa.scam_abs_denom_flow, 0.0), 0.000001))
                ) AS inflation_score
            FROM address_agg aa
            LEFT JOIN label_agg la USING (address)
            LEFT JOIN role_agg ra USING (address)
            CROSS JOIN movement_total mt
        )"
    )
}

fn eth_trader_filtered_cte() -> String {
    format!(
        "{}
        ,
        filtered AS (
            SELECT *
            FROM ranked
            WHERE (
                $2::text = 'all'
                OR ($2::text = 'leaderboard' AND scam_pool_position_count > 0)
                OR ($2::text = 'inflation' AND scam_pool_position_count > 0)
                OR ($2::text = 'custody' AND (
                    EXISTS (
                        SELECT 1
                        FROM unnest(COALESCE(pool_labels, ARRAY[]::text[])) AS label(value)
                        WHERE lower(label.value) LIKE 'custody:%'
                           OR lower(label.value) IN (
                               'risk:custody_buyer_token_confiscation',
                               'risk:holder_balance_backdoor_drain',
                               'risk:pair_balance_backdoor_drain'
                           )
                    )
                    OR EXISTS (
                        SELECT 1
                        FROM unnest(COALESCE(scam_mechanisms, ARRAY[]::text[])) AS mechanism(value)
                        WHERE lower(mechanism.value) IN (
                            'custody_buyer_token_confiscation',
                            'holder_balance_backdoor_drain',
                            'pair_balance_backdoor_drain'
                        )
                    )
                ))
                OR ($2::text = 'creators' AND (token_creator_position_count > 0 OR pool_creator_position_count > 0))
            )
            AND ($3::double precision IS NULL OR COALESCE(scam_ratio, 0.0) >= $3)
            AND ($4::bigint IS NULL OR trade_count >= $4)
            AND (
                $5::text IS NULL
                OR EXISTS (
                    SELECT 1
                    FROM unnest(COALESCE(scam_mechanisms, ARRAY[]::text[])) AS mechanism(value)
                    WHERE lower(mechanism.value) = lower($5)
                )
            )
            AND (
                $6::text IS NULL
                OR EXISTS (
                    SELECT 1
                    FROM unnest(COALESCE(scam_labels, ARRAY[]::text[]) || COALESCE(pool_labels, ARRAY[]::text[])) AS label(value)
                    WHERE lower(label.value) = lower($6)
                )
            )
            AND (
                $7::text IS NULL
                OR ($7::text = 'token_creator' AND token_creator_position_count > 0)
                OR ($7::text = 'pool_creator' AND pool_creator_position_count > 0)
                OR ($7::text = 'creator' AND (token_creator_position_count > 0 OR pool_creator_position_count > 0))
                OR ($7::text = 'custody' AND (
                    EXISTS (
                        SELECT 1
                        FROM unnest(COALESCE(pool_labels, ARRAY[]::text[])) AS label(value)
                        WHERE lower(label.value) LIKE 'custody:%'
                           OR lower(label.value) IN (
                               'risk:custody_buyer_token_confiscation',
                               'risk:holder_balance_backdoor_drain',
                               'risk:pair_balance_backdoor_drain'
                           )
                    )
                    OR EXISTS (
                        SELECT 1
                        FROM unnest(COALESCE(scam_mechanisms, ARRAY[]::text[])) AS mechanism(value)
                        WHERE lower(mechanism.value) IN (
                            'custody_buyer_token_confiscation',
                            'holder_balance_backdoor_drain',
                            'pair_balance_backdoor_drain'
                        )
                    )
                ))
            )
        )",
        eth_trader_ranked_cte()
    )
}

fn eth_trader_filtered_summary_sql() -> String {
    format!(
        "{}
        SELECT
            COUNT(*)::bigint AS address_count,
            COALESCE(SUM(pool_position_count), 0)::bigint AS pool_position_count,
            COALESCE(SUM(token_count), 0)::bigint AS token_count,
            COALESCE(SUM(scam_pool_position_count), 0)::bigint AS scam_pool_position_count,
            COALESCE(SUM(scam_token_count), 0)::bigint AS scam_token_count,
            COUNT(*) FILTER (WHERE scam_pool_position_count > 0)::bigint AS scam_address_count,
            COUNT(*) FILTER (WHERE scam_ratio = 1.0)::bigint AS all_scam_address_count,
            COUNT(*) FILTER (WHERE scam_ratio >= 0.75 AND scam_pool_position_count > 0)::bigint AS high_scam_ratio_address_count,
            COALESCE(SUM(trade_count), 0)::bigint AS trade_count,
            COALESCE(SUM(exact_trade_count), 0)::bigint AS exact_trade_count,
            COALESCE(SUM(movement_count), 0)::bigint AS movement_count,
            COALESCE(SUM(total_abs_denom_flow), 0.0)::double precision AS total_abs_denom_flow,
            COALESCE(SUM(scam_abs_denom_flow), 0.0)::double precision AS scam_abs_denom_flow,
            AVG(scam_ratio) AS avg_scam_ratio,
            AVG(scam_token_ratio) AS avg_scam_token_ratio,
            AVG(inflation_score) AS avg_inflation_score,
            MAX(latest_block) AS latest_block,
            COALESCE(BOOL_OR(movement_rows > 0), false) AS movement_rows_available
        FROM filtered",
        eth_trader_filtered_cte()
    )
}

fn eth_trader_rows_sql(sort_expression: &str) -> String {
    format!(
        "{}
        SELECT
            (ROW_NUMBER() OVER (ORDER BY {sort_expression}))::bigint AS rank,
            *
        FROM filtered
        ORDER BY {sort_expression}
        LIMIT $8 OFFSET $9",
        eth_trader_filtered_cte()
    )
}

fn eth_trader_profile_summary_sql() -> String {
    format!(
        "{}
        SELECT *
        FROM ranked
        WHERE lower(address) = lower($2)
        LIMIT 1",
        eth_trader_ranked_cte()
    )
}

fn eth_trader_positions_sql(order_by: &str) -> String {
    format!(
        r#"
        WITH movement_counts AS (
            SELECT run_id, pool_id, address, COUNT(DISTINCT tx_hash)::bigint AS exact_trade_count
            FROM token_pnl.pool_pnl_movements
            WHERE run_id = $1 AND lower(address) = lower($2)
            GROUP BY run_id, pool_id, address
        )
        SELECT
            s.pool_id,
            s.token_address,
            s.denom_address,
            s.protocol,
            s.is_scam,
            s.scam_label,
            s.scam_mechanism,
            s.lifecycle,
            s.token_creator_address,
            s.pool_creator_address,
            COALESCE(labels.pool_labels, ARRAY[]::text[]) AS pool_labels,
            COALESCE(roles.actor_roles, ARRAY[]::text[]) AS actor_role_flags,
            ARRAY(
                SELECT DISTINCT role
                FROM unnest(
                    COALESCE(roles.actor_roles, ARRAY[]::text[])
                    || ARRAY_REMOVE(ARRAY[
                        CASE WHEN lower(a.address) = lower(COALESCE(s.token_creator_address, '')) THEN 'token_creator'::text END,
                        CASE WHEN lower(a.address) = lower(COALESCE(s.pool_creator_address, '')) THEN 'pool_creator'::text END
                    ], NULL)
                ) AS role(role)
                ORDER BY role
            ) AS role_flags,
            a.position_status,
            a.valuation_status,
            a.reconciliation_status,
            a.realized_pnl_denom::double precision AS realized_pnl_denom,
            a.unrealized_value_denom::double precision AS unrealized_value_denom,
            a.unrealized_value_denom::double precision AS unrealized_pnl_denom,
            a.total_pnl_denom::double precision AS total_pnl_denom,
            a.movement_rows_retained,
            a.movement_rows_backed,
            a.is_user_candidate,
            a.accounting_context,
            lower(s.denom_address) = '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2' AS denom_is_eth,
            a.first_block,
            a.latest_block,
            COALESCE(a.movement_count, 0)::bigint AS movement_count,
            COALESCE(m.exact_trade_count, 0)::bigint AS exact_trade_count,
            COALESCE(a.denom_cashflow::double precision, 0.0) AS denom_cashflow,
            ABS(COALESCE(a.denom_cashflow::double precision, 0.0)) AS abs_denom_cashflow,
            (a.denom_in_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_in,
            (a.denom_out_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_out,
            a.token_balance::double precision AS token_balance,
            a.marked_token_value_denom::double precision AS marked_token_value_denom,
            a.pnl_proxy_denom::double precision AS pnl_proxy_denom
        FROM token_pnl.pool_address_pnl a
        JOIN token_pnl.pool_pnl_states s
          ON s.run_id = a.run_id AND s.pool_id = a.pool_id
        LEFT JOIN movement_counts m
          ON m.run_id = a.run_id AND m.pool_id = a.pool_id AND m.address = a.address
        LEFT JOIN LATERAL (
            SELECT ARRAY_AGG(label.value ORDER BY label.value) AS pool_labels
            FROM jsonb_array_elements_text(COALESCE(s.pool_labels, '[]'::jsonb)) AS label(value)
        ) labels ON true
        LEFT JOIN LATERAL (
            SELECT ARRAY_AGG(DISTINCT role.value ORDER BY role.value) AS actor_roles
            FROM jsonb_array_elements_text(COALESCE(a.actor_roles, '[]'::jsonb)) AS role(value)
        ) roles ON true
        WHERE a.run_id = $1 AND lower(a.address) = lower($2)
        ORDER BY {order_by}
        LIMIT $3
        "#
    )
}

fn eth_trader_position_detail_sql() -> String {
    eth_trader_positions_sql("s.pool_id")
        .replace(
            "WHERE a.run_id = $1 AND lower(a.address) = lower($2)",
            "WHERE a.run_id = $1 AND lower(a.address) = lower($2) AND lower(a.pool_id) = lower($3)",
        )
        .replace("LIMIT $3", "LIMIT 1")
}

fn eth_trader_mechanism_breakdown_sql() -> String {
    r#"
    WITH movement_counts AS (
        SELECT run_id, pool_id, address, COUNT(DISTINCT tx_hash)::bigint AS exact_trade_count
        FROM token_pnl.pool_pnl_movements
        WHERE run_id = $1 AND lower(address) = lower($2)
        GROUP BY run_id, pool_id, address
    ),
    rows AS (
        SELECT
            COALESCE(NULLIF(s.scam_mechanism, ''), 'unclassified') AS mechanism,
            s.token_address,
            s.is_scam,
            COALESCE(a.movement_count, 0)::bigint AS movement_count,
            COALESCE(m.exact_trade_count, 0)::bigint AS exact_trade_count,
            ABS(COALESCE(a.denom_cashflow::double precision, 0.0)) AS abs_denom_cashflow
        FROM token_pnl.pool_address_pnl a
        JOIN token_pnl.pool_pnl_states s
          ON s.run_id = a.run_id AND s.pool_id = a.pool_id
        LEFT JOIN movement_counts m
          ON m.run_id = a.run_id AND m.pool_id = a.pool_id AND m.address = a.address
        WHERE a.run_id = $1 AND lower(a.address) = lower($2)
    )
    SELECT
        mechanism,
        COUNT(*)::bigint AS pool_position_count,
        COUNT(DISTINCT token_address)::bigint AS token_count,
        COUNT(*) FILTER (WHERE is_scam)::bigint AS scam_pool_position_count,
        COUNT(DISTINCT token_address) FILTER (WHERE is_scam)::bigint AS scam_token_count,
        COALESCE(SUM(CASE WHEN exact_trade_count > 0 THEN exact_trade_count ELSE movement_count END), 0)::bigint AS trade_count,
        COALESCE(SUM(abs_denom_cashflow), 0.0)::double precision AS total_abs_denom_flow,
        COALESCE(SUM(CASE WHEN is_scam THEN abs_denom_cashflow ELSE 0.0 END), 0.0)::double precision AS scam_abs_denom_flow
    FROM rows
    GROUP BY mechanism
    ORDER BY scam_pool_position_count DESC, scam_abs_denom_flow DESC, trade_count DESC, mechanism
    LIMIT $3
    "#
    .to_string()
}

fn eth_trader_label_breakdown_sql() -> String {
    r#"
    WITH movement_counts AS (
        SELECT run_id, pool_id, address, COUNT(DISTINCT tx_hash)::bigint AS exact_trade_count
        FROM token_pnl.pool_pnl_movements
        WHERE run_id = $1 AND lower(address) = lower($2)
        GROUP BY run_id, pool_id, address
    ),
    base AS (
        SELECT
            s.token_address,
            s.is_scam,
            s.scam_label,
            s.pool_labels,
            COALESCE(a.movement_count, 0)::bigint AS movement_count,
            COALESCE(m.exact_trade_count, 0)::bigint AS exact_trade_count,
            ABS(COALESCE(a.denom_cashflow::double precision, 0.0)) AS abs_denom_cashflow
        FROM token_pnl.pool_address_pnl a
        JOIN token_pnl.pool_pnl_states s
          ON s.run_id = a.run_id AND s.pool_id = a.pool_id
        LEFT JOIN movement_counts m
          ON m.run_id = a.run_id AND m.pool_id = a.pool_id AND m.address = a.address
        WHERE a.run_id = $1 AND lower(a.address) = lower($2)
    ),
    label_rows AS (
        SELECT
            'scam_label'::text AS kind,
            scam_label AS label,
            token_address,
            is_scam,
            movement_count,
            exact_trade_count,
            abs_denom_cashflow
        FROM base
        WHERE scam_label IS NOT NULL AND scam_label <> ''
        UNION ALL
        SELECT
            'pool_label'::text AS kind,
            label.value AS label,
            base.token_address,
            base.is_scam,
            base.movement_count,
            base.exact_trade_count,
            base.abs_denom_cashflow
        FROM base
        JOIN LATERAL jsonb_array_elements_text(COALESCE(base.pool_labels, '[]'::jsonb)) AS label(value) ON true
    )
    SELECT
        kind,
        label,
        COUNT(*)::bigint AS pool_position_count,
        COUNT(DISTINCT token_address)::bigint AS token_count,
        COUNT(*) FILTER (WHERE is_scam)::bigint AS scam_pool_position_count,
        COUNT(DISTINCT token_address) FILTER (WHERE is_scam)::bigint AS scam_token_count,
        COALESCE(SUM(CASE WHEN exact_trade_count > 0 THEN exact_trade_count ELSE movement_count END), 0)::bigint AS trade_count,
        COALESCE(SUM(abs_denom_cashflow), 0.0)::double precision AS total_abs_denom_flow,
        COALESCE(SUM(CASE WHEN is_scam THEN abs_denom_cashflow ELSE 0.0 END), 0.0)::double precision AS scam_abs_denom_flow
    FROM label_rows
    GROUP BY kind, label
    ORDER BY scam_pool_position_count DESC, scam_abs_denom_flow DESC, trade_count DESC, kind, label
    LIMIT $3
    "#
    .to_string()
}

fn eth_trader_recent_movements_sql() -> String {
    r#"
    SELECT
        m.pool_id,
        s.token_address,
        s.denom_address,
        s.protocol,
        s.is_scam,
        s.scam_label,
        s.scam_mechanism,
        m.entry_index,
        m.tx_hash,
        m.block_number,
        m.block_timestamp,
        m.tx_index,
        m.log_index,
        m.kind,
        m.pool_direct,
        (m.token_in_raw / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_in,
        (m.token_out_raw / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_out,
        (m.denom_in_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_in,
        (m.denom_out_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_out,
        ((m.denom_out_raw - m.denom_in_raw) / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_delta,
        ((m.token_out_raw - m.token_in_raw) / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_delta,
        (m.native_fee_raw / POWER(10::numeric, 18))::double precision AS native_fee_eth,
        (m.native_priority_fee_raw / POWER(10::numeric, 18))::double precision AS native_priority_fee_eth
    FROM token_pnl.pool_pnl_movements m
    JOIN token_pnl.pool_pnl_states s
      ON s.run_id = m.run_id AND s.pool_id = m.pool_id
    WHERE m.run_id = $1 AND lower(m.address) = lower($2)
    ORDER BY m.block_number DESC, m.tx_index DESC, m.entry_index DESC
    LIMIT $3
    "#
    .to_string()
}

fn eth_trader_trade_movements_sql() -> String {
    r#"
    SELECT
        m.pool_id,
        s.token_address,
        s.denom_address,
        s.protocol,
        s.is_scam,
        s.scam_label,
        s.scam_mechanism,
        m.entry_index,
        m.tx_hash,
        m.block_number,
        m.block_timestamp,
        m.tx_index,
        m.log_index,
        m.kind,
        m.pool_direct,
        (m.token_in_raw / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_in,
        (m.token_out_raw / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_out,
        (m.denom_in_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_in,
        (m.denom_out_raw / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_out,
        ((m.denom_out_raw - m.denom_in_raw) / POWER(10::numeric, GREATEST(s.denom_decimals::int, 0)))::double precision AS denom_delta,
        ((m.token_out_raw - m.token_in_raw) / POWER(10::numeric, GREATEST(s.token_decimals::int, 0)))::double precision AS token_delta,
        (m.native_fee_raw / POWER(10::numeric, 18))::double precision AS native_fee_eth,
        (m.native_priority_fee_raw / POWER(10::numeric, 18))::double precision AS native_priority_fee_eth
    FROM token_pnl.pool_pnl_movements m
    JOIN token_pnl.pool_pnl_states s
      ON s.run_id = m.run_id AND s.pool_id = m.pool_id
    WHERE m.run_id = $1 AND lower(m.address) = lower($2) AND lower(m.pool_id) = lower($3)
    ORDER BY m.block_number DESC, m.tx_index DESC, m.entry_index DESC
    LIMIT $4
    "#
    .to_string()
}

fn eth_trader_summary_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    Ok(json!({
        "address_count": row.try_get::<i64, _>("address_count")?,
        "pool_position_count": row.try_get::<i64, _>("pool_position_count")?,
        "token_count": row.try_get::<i64, _>("token_count")?,
        "scam_pool_position_count": row.try_get::<i64, _>("scam_pool_position_count")?,
        "scam_token_count": row.try_get::<i64, _>("scam_token_count")?,
        "scam_address_count": row.try_get::<i64, _>("scam_address_count")?,
        "all_scam_address_count": row.try_get::<i64, _>("all_scam_address_count")?,
        "high_scam_ratio_address_count": row.try_get::<i64, _>("high_scam_ratio_address_count")?,
        "trade_count": row.try_get::<i64, _>("trade_count")?,
        "exact_trade_count": row.try_get::<i64, _>("exact_trade_count")?,
        "movement_count": row.try_get::<i64, _>("movement_count")?,
        "total_abs_denom_flow": row.try_get::<f64, _>("total_abs_denom_flow")?,
        "scam_abs_denom_flow": row.try_get::<f64, _>("scam_abs_denom_flow")?,
        "avg_scam_ratio": row.try_get::<Option<f64>, _>("avg_scam_ratio")?,
        "avg_scam_token_ratio": row.try_get::<Option<f64>, _>("avg_scam_token_ratio")?,
        "avg_inflation_score": row.try_get::<Option<f64>, _>("avg_inflation_score")?,
        "latest_block": row.try_get::<Option<i64>, _>("latest_block")?,
        "movement_rows_available": row.try_get::<bool, _>("movement_rows_available")?,
    }))
}

fn eth_trader_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    Ok(json!({
        "rank": row.try_get::<i64, _>("rank").ok(),
        "address": row.try_get::<String, _>("address")?,
        "inflation_score": row.try_get::<Option<f64>, _>("inflation_score")?,
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

/// Address-level aggregate PnL band for the profile/activity page. Reads the
/// SUM columns added to `address_agg` (flow through `ranked` via `aa.*`).
fn eth_address_aggregate_json(row: &sqlx::postgres::PgRow) -> Result<Value> {
    let all_eth = row.try_get::<Option<bool>, _>("all_denom_eth")?.unwrap_or(false);
    let realized = row.try_get::<Option<f64>, _>("realized_pnl_denom_sum")?;
    let unrealized = row.try_get::<Option<f64>, _>("unrealized_pnl_denom_sum")?;
    let total = row.try_get::<Option<f64>, _>("total_pnl_denom_sum")?;
    Ok(json!({
        "realized_pnl_denom": realized,
        "unrealized_pnl_denom": unrealized,
        "total_pnl_denom": total,
        "realized_pnl_eth": if all_eth { realized } else { None },
        "unrealized_pnl_eth": if all_eth { unrealized } else { None },
        "total_pnl_eth": if all_eth { total } else { None },
        "denom_is_eth": all_eth,
        "net_denom_cashflow": row.try_get::<Option<f64>, _>("net_denom_cashflow")?,
        "denom_in": row.try_get::<Option<f64>, _>("denom_in")?,
        "denom_out": row.try_get::<Option<f64>, _>("denom_out")?,
        "gas_paid_eth": row.try_get::<Option<f64>, _>("gas_paid")?,
        "pool_count": row.try_get::<i64, _>("pool_position_count")?,
        "scam_pool_count": row.try_get::<i64, _>("scam_pool_position_count")?,
        "win_count": row.try_get::<i64, _>("win_count")?,
        "loss_count": row.try_get::<i64, _>("loss_count")?,
        "breakeven_count": row.try_get::<i64, _>("breakeven_count")?,
    }))
}

/// Derive a per-address `type` + stackable validity flags from DB-only signals
/// (roles, scam-share, gas, reconciliation). `confidence = "db_only"`; the
/// EOA-vs-contract / known-router refinement is a chain-enriched fast-follow.
fn eth_address_type_and_flags(row: &sqlx::postgres::PgRow) -> Result<(Value, Value)> {
    let role_flags = row.try_get::<Vec<String>, _>("role_flags").unwrap_or_default();
    let scam_ratio = row.try_get::<Option<f64>, _>("scam_ratio")?.unwrap_or(0.0);
    let pool_count = row.try_get::<i64, _>("pool_position_count")?;
    let total_pnl = row.try_get::<Option<f64>, _>("total_pnl_denom_sum")?.unwrap_or(0.0);
    let gas_paid = row.try_get::<Option<f64>, _>("gas_paid")?.unwrap_or(0.0);
    let movement_count = row.try_get::<i64, _>("movement_count")?;
    let total_abs = row.try_get::<Option<f64>, _>("total_abs_denom_flow")?.unwrap_or(0.0);
    let scam_abs = row.try_get::<Option<f64>, _>("scam_abs_denom_flow")?.unwrap_or(0.0);
    let movement_backed = row.try_get::<i64, _>("movement_backed_position_count")?;
    let token_creator = row.try_get::<i64, _>("token_creator_position_count")?;
    let pool_creator = row.try_get::<i64, _>("pool_creator_position_count")?;

    let has = |role: &str| role_flags.iter().any(|value| value == role);
    let is_creator = token_creator > 0 || pool_creator > 0 || has("token_creator") || has("pool_creator");

    let address_type = if is_creator && scam_ratio >= 0.5 {
        "creator_scammer"
    } else if is_creator {
        "creator"
    } else if has("external_token_source") && has("seller") && !has("buyer") {
        "external_inflow_seller"
    } else if has("custody_victim_candidate") && total_pnl > 0.0 {
        "custody_anomaly"
    } else if scam_ratio >= 0.8 {
        "fresh_launch_sniper"
    } else if has("buyer") && scam_ratio < 0.5 {
        "clean_trader"
    } else {
        "mixed_trader"
    };

    let mut flags: Vec<String> = Vec::new();
    if total_abs > 0.0 && scam_abs / total_abs >= 0.8 {
        flags.push("pnl_dominated_by_scam_pools".to_string());
    }
    if gas_paid == 0.0 && movement_count > 0 {
        flags.push("gas_not_attributed".to_string());
    }
    if movement_backed < pool_count {
        flags.push("movement_reconciliation_incomplete".to_string());
    }
    if has("custody_victim_candidate") && total_pnl > 0.0 {
        flags.push("custody_victim_with_positive_pnl".to_string());
    }

    Ok((
        json!({ "type": address_type, "confidence": "db_only" }),
        json!(flags),
    ))
}

fn eth_trader_pool_position_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    let pool_labels = row.try_get::<Vec<String>, _>("pool_labels")?;
    let role_flags = row.try_get::<Vec<String>, _>("role_flags")?;
    let actor_role_flags = row
        .try_get::<Vec<String>, _>("actor_role_flags")
        .unwrap_or_default();
    let actor_roles = actor_role_flags
        .iter()
        .map(|role| {
            json!({
                "role": role,
                "source": "pnl_accounting",
            })
        })
        .collect::<Vec<_>>();
    let scam_label = row.try_get::<Option<String>, _>("scam_label")?;
    let scam_mechanism = row.try_get::<Option<String>, _>("scam_mechanism")?;
    let lifecycle = row.try_get::<Option<String>, _>("lifecycle")?;
    let position_status = row.try_get::<String, _>("position_status")?;
    let valuation_status = row.try_get::<String, _>("valuation_status")?;
    let reconciliation_status = row.try_get::<String, _>("reconciliation_status")?;
    let denom_is_eth = row.try_get::<bool, _>("denom_is_eth").unwrap_or(false);
    let realized_pnl_denom = row.try_get::<Option<f64>, _>("realized_pnl_denom")?;
    let unrealized_pnl_denom = row.try_get::<Option<f64>, _>("unrealized_pnl_denom")?;
    let total_pnl_denom = row.try_get::<Option<f64>, _>("total_pnl_denom")?;
    let labels = merged_labels(
        &pool_labels,
        &role_flags,
        [
            scam_label.as_deref(),
            scam_mechanism.as_deref(),
            lifecycle.as_deref(),
            Some(position_status.as_str()),
            Some(valuation_status.as_str()),
            Some(reconciliation_status.as_str()),
        ],
    );

    Ok(json!({
        "pool_id": row.try_get::<String, _>("pool_id")?,
        "token_address": row.try_get::<String, _>("token_address")?,
        "denom_address": row.try_get::<String, _>("denom_address")?,
        "protocol": row.try_get::<Option<String>, _>("protocol")?,
        "is_scam": row.try_get::<bool, _>("is_scam")?,
        "scam_label": scam_label,
        "scam_mechanism": scam_mechanism,
        "lifecycle": lifecycle,
        "token_creator_address": row.try_get::<Option<String>, _>("token_creator_address")?,
        "pool_creator_address": row.try_get::<Option<String>, _>("pool_creator_address")?,
        "pool_labels": pool_labels,
        "role_flags": role_flags,
        "actor_roles": actor_roles,
        "labels": labels,
        "position_status": position_status,
        "valuation_status": valuation_status,
        "reconciliation_status": reconciliation_status,
        "movement_backed_status": reconciliation_status,
        "movement_rows_retained": row.try_get::<i64, _>("movement_rows_retained")?,
        "movement_backed": row.try_get::<bool, _>("movement_rows_backed")?,
        "movement_rows_backed": row.try_get::<bool, _>("movement_rows_backed")?,
        "is_user_candidate": row.try_get::<bool, _>("is_user_candidate")?,
        "accounting_context": row.try_get::<Value, _>("accounting_context")?,
        "first_block": row.try_get::<Option<i64>, _>("first_block")?,
        "latest_block": row.try_get::<Option<i64>, _>("latest_block")?,
        "movement_count": row.try_get::<i64, _>("movement_count")?,
        "exact_trade_count": row.try_get::<i64, _>("exact_trade_count")?,
        "denom_cashflow": row.try_get::<f64, _>("denom_cashflow")?,
        "abs_denom_cashflow": row.try_get::<f64, _>("abs_denom_cashflow")?,
        "denom_in": row.try_get::<Option<f64>, _>("denom_in")?,
        "denom_out": row.try_get::<Option<f64>, _>("denom_out")?,
        "token_balance": row.try_get::<Option<f64>, _>("token_balance")?,
        "marked_token_value_denom": row.try_get::<Option<f64>, _>("marked_token_value_denom")?,
        "pnl_proxy_denom": row.try_get::<Option<f64>, _>("pnl_proxy_denom")?,
        "realized_pnl_denom": realized_pnl_denom,
        "unrealized_value_denom": row.try_get::<Option<f64>, _>("unrealized_value_denom")?,
        "unrealized_pnl_denom": unrealized_pnl_denom,
        "total_pnl_denom": total_pnl_denom,
        "realized_pnl_eth": if denom_is_eth { realized_pnl_denom } else { None },
        "unrealized_pnl_eth": if denom_is_eth { unrealized_pnl_denom } else { None },
        "total_pnl_eth": if denom_is_eth { total_pnl_denom } else { None },
    }))
}

fn eth_trader_mechanism_breakdown_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    Ok(json!({
        "mechanism": row.try_get::<String, _>("mechanism")?,
        "pool_position_count": row.try_get::<i64, _>("pool_position_count")?,
        "token_count": row.try_get::<i64, _>("token_count")?,
        "scam_pool_position_count": row.try_get::<i64, _>("scam_pool_position_count")?,
        "scam_token_count": row.try_get::<i64, _>("scam_token_count")?,
        "trade_count": row.try_get::<i64, _>("trade_count")?,
        "total_abs_denom_flow": row.try_get::<f64, _>("total_abs_denom_flow")?,
        "scam_abs_denom_flow": row.try_get::<f64, _>("scam_abs_denom_flow")?,
    }))
}

fn eth_trader_label_breakdown_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    Ok(json!({
        "kind": row.try_get::<String, _>("kind")?,
        "label": row.try_get::<String, _>("label")?,
        "pool_position_count": row.try_get::<i64, _>("pool_position_count")?,
        "token_count": row.try_get::<i64, _>("token_count")?,
        "scam_pool_position_count": row.try_get::<i64, _>("scam_pool_position_count")?,
        "scam_token_count": row.try_get::<i64, _>("scam_token_count")?,
        "trade_count": row.try_get::<i64, _>("trade_count")?,
        "total_abs_denom_flow": row.try_get::<f64, _>("total_abs_denom_flow")?,
        "scam_abs_denom_flow": row.try_get::<f64, _>("scam_abs_denom_flow")?,
    }))
}

fn eth_trader_movement_row(row: sqlx::postgres::PgRow) -> Result<Value> {
    Ok(json!({
        "pool_id": row.try_get::<String, _>("pool_id")?,
        "token_address": row.try_get::<String, _>("token_address")?,
        "denom_address": row.try_get::<String, _>("denom_address")?,
        "protocol": row.try_get::<Option<String>, _>("protocol")?,
        "is_scam": row.try_get::<bool, _>("is_scam")?,
        "scam_label": row.try_get::<Option<String>, _>("scam_label")?,
        "scam_mechanism": row.try_get::<Option<String>, _>("scam_mechanism")?,
        "entry_index": row.try_get::<i64, _>("entry_index")?,
        "tx_hash": row.try_get::<String, _>("tx_hash")?,
        "block_number": row.try_get::<i64, _>("block_number")?,
        "block_timestamp": row.try_get::<i64, _>("block_timestamp")?,
        "tx_index": row.try_get::<i64, _>("tx_index")?,
        "log_index": row.try_get::<Option<i64>, _>("log_index")?,
        "kind": row.try_get::<String, _>("kind")?,
        "pool_direct": row.try_get::<bool, _>("pool_direct")?,
        "token_in": row.try_get::<Option<f64>, _>("token_in")?,
        "token_out": row.try_get::<Option<f64>, _>("token_out")?,
        "token_delta": row.try_get::<Option<f64>, _>("token_delta")?,
        "denom_in": row.try_get::<Option<f64>, _>("denom_in")?,
        "denom_out": row.try_get::<Option<f64>, _>("denom_out")?,
        "denom_delta": row.try_get::<Option<f64>, _>("denom_delta")?,
        "native_fee_eth": row.try_get::<Option<f64>, _>("native_fee_eth")?,
        "native_priority_fee_eth": row.try_get::<Option<f64>, _>("native_priority_fee_eth")?,
    }))
}

fn merged_labels<'a>(
    pool_labels: &[String],
    role_flags: &[String],
    optional_labels: impl IntoIterator<Item = Option<&'a str>>,
) -> Vec<String> {
    let mut labels = pool_labels
        .iter()
        .chain(role_flags.iter())
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    for label in optional_labels.into_iter().flatten() {
        let label = label.trim();
        if !label.is_empty() {
            labels.push(label.to_string());
        }
    }
    labels.sort();
    labels.dedup();
    labels
}

fn row_to_run(row: sqlx::postgres::PgRow) -> Result<RiskAtlasRun> {
    Ok(RiskAtlasRun {
        run_id: row.try_get("run_id")?,
        chain: row.try_get("chain")?,
        source_kind: row.try_get("source_kind")?,
        source_ref: row.try_get("source_ref")?,
        start_block: row.try_get("start_block")?,
        end_block: row.try_get("end_block")?,
        block_count: row.try_get("block_count")?,
        token_count: row.try_get("token_count")?,
        pool_count: row.try_get("pool_count")?,
        scam_label_count: row.try_get("scam_label_count")?,
        direct_lp_feature_row_count: row.try_get("direct_lp_feature_row_count")?,
        active_observation_row_count: row.try_get("active_observation_row_count")?,
        status: row.try_get("status")?,
        generated_at: row.try_get("generated_at")?,
        metadata: row.try_get::<Value, _>("metadata")?,
    })
}

fn row_to_distribution(row: sqlx::postgres::PgRow) -> Result<DistributionBucket> {
    let section: String = row.try_get("section")?;
    Ok(DistributionBucket {
        section: canonical_distribution_section(&section),
        bucket: row.try_get("bucket")?,
        count: row.try_get("count")?,
        share: row.try_get("share")?,
        sort_order: row.try_get("sort_order")?,
    })
}

fn row_to_numeric_stat(row: sqlx::postgres::PgRow) -> Result<NumericStat> {
    let metric: String = row.try_get("metric")?;
    Ok(NumericStat {
        section: row.try_get("section")?,
        metric: canonical_numeric_metric(&metric),
        count: row.try_get("count")?,
        min: row.try_get("min")?,
        p25: row.try_get("p25")?,
        median: row.try_get("median")?,
        p75: row.try_get("p75")?,
        p90: row.try_get("p90")?,
        p95: row.try_get("p95")?,
        max: row.try_get("max")?,
        sort_order: row.try_get("sort_order")?,
    })
}

fn canonical_distribution_section(section: &str) -> String {
    if section.starts_with("direct_lp_target_") && section.ends_with("_active_observations") {
        section.replace("_active_observations", "_active_observation_delta")
    } else {
        section.to_string()
    }
}

fn canonical_numeric_metric(metric: &str) -> String {
    match metric {
        "Trading Enabled To Label Blocks" => {
            "Trading Enabled To Label Chain Block Delta".to_string()
        }
        "First Pre-Removal LP Approval To Removal Blocks" => {
            "First Pre-Removal LP Approval To Removal Chain Block Delta".to_string()
        }
        "Last Pre-Removal LP Approval To Removal Blocks" => {
            "Last Pre-Removal LP Approval To Removal Chain Block Delta".to_string()
        }
        _ => metric.to_string(),
    }
}

fn row_to_active_target(row: sqlx::postgres::PgRow) -> Result<ActiveTargetSummary> {
    let row_kind: String = row.try_get("row_kind")?;
    Ok(ActiveTargetSummary {
        row_kind: canonical_row_kind(&row_kind),
        active_observation_delta: row.try_get("horizon_active_observations")?,
        rows: row.try_get("rows")?,
        unique_pools: row.try_get("unique_pools")?,
        positives: row.try_get("positives")?,
        negatives: row.try_get("negatives")?,
        sort_order: row.try_get("sort_order")?,
    })
}

fn canonical_row_kind(row_kind: &str) -> String {
    if row_kind.ends_with("_within_active_observations") {
        row_kind.replace(
            "_within_active_observations",
            "_within_active_observation_delta",
        )
    } else {
        row_kind.to_string()
    }
}

fn row_to_event_evidence(row: sqlx::postgres::PgRow) -> Result<EventEvidenceRow> {
    Ok(EventEvidenceRow {
        token_address: row.try_get("token_address")?,
        pool_address: row.try_get("pool_address")?,
        protocol: row.try_get("protocol")?,
        event_kind: row.try_get("event_kind")?,
        mechanism: row.try_get("mechanism")?,
        block_number: row.try_get("block_number")?,
        block_timestamp: row.try_get("block_timestamp")?,
        tx_hash: row.try_get("tx_hash")?,
        mempool_first_seen_ms: row.try_get("mempool_first_seen_ms")?,
        source: row.try_get("source")?,
        sort_order: row.try_get("sort_order")?,
    })
}

fn row_to_decision_question(row: sqlx::postgres::PgRow) -> Result<DecisionQuestion> {
    Ok(DecisionQuestion {
        question_id: row.try_get("question_id")?,
        category: row.try_get("category")?,
        question: row.try_get("question")?,
        headline: row.try_get("headline")?,
        answer: row.try_get("answer")?,
        status: row.try_get("status")?,
        denominator_label: row.try_get("denominator_label")?,
        denominator_count: row.try_get("denominator_count")?,
        payload: row.try_get::<Value, _>("payload")?,
        sort_order: row.try_get("sort_order")?,
    })
}

fn row_to_review_example(row: sqlx::postgres::PgRow) -> Result<ReviewExample> {
    Ok(ReviewExample {
        queue: row.try_get("queue")?,
        token_address: row.try_get("token_address")?,
        pool_address: row.try_get("pool_address")?,
        protocol: row.try_get("protocol")?,
        symbol: row.try_get("symbol")?,
        mechanism: row.try_get("mechanism")?,
        mechanism_label: row.try_get("mechanism_label")?,
        label_block: row.try_get("label_block")?,
        trading_enabled_block: row.try_get("trading_enabled_block")?,
        trading_enabled_to_label_chain_block_delta: row.try_get("age_blocks")?,
        liquidity_eth: row.try_get("liquidity_eth")?,
        price_ratio_to_initial: row.try_get("price_ratio_to_initial")?,
        evidence_summary: row.try_get("evidence_summary")?,
        evidence_ref: row.try_get("evidence_ref")?,
        sort_order: row.try_get("sort_order")?,
    })
}

fn row_to_model_readiness(row: sqlx::postgres::PgRow) -> Result<ModelReadinessItem> {
    Ok(ModelReadinessItem {
        name: row.try_get("name")?,
        status: row.try_get("status")?,
        detail: row.try_get("detail")?,
        sort_order: row.try_get("sort_order")?,
    })
}
