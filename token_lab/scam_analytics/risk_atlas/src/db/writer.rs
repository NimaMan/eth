use eyre::Result;
use serde_json::to_value;
use sqlx::PgPool;

use crate::api::RiskAtlasPageView;
use crate::ingest::report::RiskAtlasReportImport;

use super::schema::{PoolEligibilityRow, RiskAtlasRun};

#[derive(Clone)]
pub struct RiskAtlasWriter {
    pool: PgPool,
}

impl RiskAtlasWriter {
    pub async fn connect(database_url: &str) -> Result<Self> {
        Ok(Self {
            pool: PgPool::connect(database_url).await?,
        })
    }

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn replace_report_import(&self, import: &RiskAtlasReportImport) -> Result<()> {
        self.upsert_run(&import.run).await?;

        let mut tx = self.pool.begin().await?;
        for table in [
            "risk_atlas_distributions",
            "risk_atlas_pool_eligibility",
            "risk_atlas_numeric_stats",
            "risk_atlas_active_targets",
            "risk_atlas_review_examples",
            "risk_atlas_model_readiness",
            "risk_atlas_page_snapshots",
        ] {
            let query = format!("DELETE FROM {table} WHERE run_id = $1");
            sqlx::query(&query)
                .bind(&import.run.run_id)
                .execute(&mut *tx)
                .await?;
        }

        for item in &import.distributions {
            sqlx::query(
                r#"
                INSERT INTO risk_atlas_distributions (
                    run_id, section, bucket, count, share, sort_order
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(&import.run.run_id)
            .bind(&item.section)
            .bind(&item.bucket)
            .bind(item.count)
            .bind(item.share)
            .bind(item.sort_order)
            .execute(&mut *tx)
            .await?;
        }

        for item in &import.pool_eligibility {
            insert_pool_eligibility_row(&mut tx, &import.run.run_id, item).await?;
        }

        for item in &import.numeric_stats {
            sqlx::query(
                r#"
                INSERT INTO risk_atlas_numeric_stats (
                    run_id, section, metric, count, min, p25, median, p75, p90, p95, max, sort_order
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                "#,
            )
            .bind(&import.run.run_id)
            .bind(&item.section)
            .bind(&item.metric)
            .bind(item.count)
            .bind(item.min)
            .bind(item.p25)
            .bind(item.median)
            .bind(item.p75)
            .bind(item.p90)
            .bind(item.p95)
            .bind(item.max)
            .bind(item.sort_order)
            .execute(&mut *tx)
            .await?;
        }

        for item in &import.active_targets {
            let target_key = active_target_key(&item.row_kind, item.horizon_active_observations);
            sqlx::query(
                r#"
                INSERT INTO risk_atlas_active_targets (
                    run_id, target_key, row_kind, horizon_active_observations, rows,
                    unique_pools, positives, negatives, sort_order
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(&import.run.run_id)
            .bind(target_key)
            .bind(&item.row_kind)
            .bind(item.horizon_active_observations)
            .bind(item.rows)
            .bind(item.unique_pools)
            .bind(item.positives)
            .bind(item.negatives)
            .bind(item.sort_order)
            .execute(&mut *tx)
            .await?;
        }

        for item in &import.review_examples {
            sqlx::query(
                r#"
                INSERT INTO risk_atlas_review_examples (
                    run_id, queue, token_address, pool_address, protocol, symbol, mechanism,
                    mechanism_label, label_block, trading_enabled_block, age_blocks, liquidity_eth,
                    price_ratio_to_initial, evidence_summary, evidence_ref, sort_order
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
                "#,
            )
            .bind(&import.run.run_id)
            .bind(&item.queue)
            .bind(&item.token_address)
            .bind(&item.pool_address)
            .bind(&item.protocol)
            .bind(&item.symbol)
            .bind(&item.mechanism)
            .bind(&item.mechanism_label)
            .bind(item.label_block)
            .bind(item.trading_enabled_block)
            .bind(item.age_blocks)
            .bind(item.liquidity_eth)
            .bind(item.price_ratio_to_initial)
            .bind(&item.evidence_summary)
            .bind(&item.evidence_ref)
            .bind(item.sort_order)
            .execute(&mut *tx)
            .await?;
        }

        for item in &import.model_readiness {
            sqlx::query(
                r#"
                INSERT INTO risk_atlas_model_readiness (
                    run_id, name, status, detail, sort_order
                )
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(&import.run.run_id)
            .bind(&item.name)
            .bind(&item.status)
            .bind(&item.detail)
            .bind(item.sort_order)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn upsert_run(&self, run: &RiskAtlasRun) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO risk_atlas_runs (
                run_id, chain, source_kind, source_ref, start_block, end_block,
                block_count, token_count, pool_count, scam_label_count,
                direct_lp_feature_row_count, active_observation_row_count,
                status, generated_at, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (run_id) DO UPDATE SET
                chain = EXCLUDED.chain,
                source_kind = EXCLUDED.source_kind,
                source_ref = EXCLUDED.source_ref,
                start_block = EXCLUDED.start_block,
                end_block = EXCLUDED.end_block,
                block_count = EXCLUDED.block_count,
                token_count = EXCLUDED.token_count,
                pool_count = EXCLUDED.pool_count,
                scam_label_count = EXCLUDED.scam_label_count,
                direct_lp_feature_row_count = EXCLUDED.direct_lp_feature_row_count,
                active_observation_row_count = EXCLUDED.active_observation_row_count,
                status = EXCLUDED.status,
                generated_at = EXCLUDED.generated_at,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(&run.run_id)
        .bind(&run.chain)
        .bind(&run.source_kind)
        .bind(&run.source_ref)
        .bind(run.start_block)
        .bind(run.end_block)
        .bind(run.block_count)
        .bind(run.token_count)
        .bind(run.pool_count)
        .bind(run.scam_label_count)
        .bind(run.direct_lp_feature_row_count)
        .bind(run.active_observation_row_count)
        .bind(&run.status)
        .bind(run.generated_at)
        .bind(&run.metadata)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn write_page_snapshot(
        &self,
        snapshot_id: &str,
        title: &str,
        view: &RiskAtlasPageView,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO risk_atlas_page_snapshots (
                snapshot_id, run_id, title, summary, sections, review_queues, model_readiness
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (snapshot_id) DO UPDATE SET
                run_id = EXCLUDED.run_id,
                title = EXCLUDED.title,
                summary = EXCLUDED.summary,
                sections = EXCLUDED.sections,
                review_queues = EXCLUDED.review_queues,
                model_readiness = EXCLUDED.model_readiness,
                generated_at = now()
            "#,
        )
        .bind(snapshot_id)
        .bind(&view.run.run_id)
        .bind(title)
        .bind(to_value(&view.run)?)
        .bind(to_value(&view.sections)?)
        .bind(to_value(&view.review_examples)?)
        .bind(to_value(&view.model_readiness)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn active_target_key(row_kind: &str, horizon: Option<i32>) -> String {
    match horizon {
        Some(horizon) => format!("{row_kind}:{horizon}"),
        None => row_kind.to_string(),
    }
}

async fn insert_pool_eligibility_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: &str,
    item: &PoolEligibilityRow,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO risk_atlas_pool_eligibility (
            run_id, token_address, pool_address, protocol, quote_symbol,
            eligible, eligibility_block, eligibility_liquidity,
            first_observed_block, last_observed_block
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(run_id)
    .bind(&item.token_address)
    .bind(&item.pool_address)
    .bind(&item.protocol)
    .bind(&item.quote_symbol)
    .bind(item.eligible)
    .bind(item.eligibility_block)
    .bind(item.eligibility_liquidity)
    .bind(item.first_observed_block)
    .bind(item.last_observed_block)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
