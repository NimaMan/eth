use eyre::Result;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use crate::api::RiskAtlasPageView;
use crate::atlas::default_story_sections;

use super::schema::{
    ActiveTargetSummary, DistributionBucket, ModelReadinessItem, NumericStat, ReviewExample,
    RiskAtlasRun,
};

#[derive(Clone)]
pub struct RiskAtlasReader {
    pool: PgPool,
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

    pub async fn page_view(&self, run_id: &str) -> Result<Option<RiskAtlasPageView>> {
        let Some(run) = self.run(run_id).await? else {
            return Ok(None);
        };

        Ok(Some(RiskAtlasPageView {
            run,
            sections: default_story_sections(),
            distributions: self.distributions(run_id).await?,
            numeric_stats: self.numeric_stats(run_id).await?,
            active_targets: self.active_targets(run_id).await?,
            review_examples: self.review_examples(run_id).await?,
            model_readiness: self.model_readiness(run_id).await?,
        }))
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
    Ok(DistributionBucket {
        section: row.try_get("section")?,
        bucket: row.try_get("bucket")?,
        count: row.try_get("count")?,
        share: row.try_get("share")?,
        sort_order: row.try_get("sort_order")?,
    })
}

fn row_to_numeric_stat(row: sqlx::postgres::PgRow) -> Result<NumericStat> {
    Ok(NumericStat {
        section: row.try_get("section")?,
        metric: row.try_get("metric")?,
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

fn row_to_active_target(row: sqlx::postgres::PgRow) -> Result<ActiveTargetSummary> {
    Ok(ActiveTargetSummary {
        row_kind: row.try_get("row_kind")?,
        horizon_active_observations: row.try_get("horizon_active_observations")?,
        rows: row.try_get("rows")?,
        unique_pools: row.try_get("unique_pools")?,
        positives: row.try_get("positives")?,
        negatives: row.try_get("negatives")?,
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
        age_blocks: row.try_get("age_blocks")?,
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
