use eyre::Result;
use serde_json::Value;
use sqlx::Row;

use crate::api::RiskAtlasPageView;
use crate::atlas::{build_page_story, default_story_sections};
use crate::db::schema::{
    ActiveTargetSummary, DecisionQuestion, DistributionBucket, EventEvidenceRow,
    ModelReadinessItem, NumericStat, ReviewExample, RiskAtlasRun,
};

use super::RiskAtlasReader;

impl RiskAtlasReader {
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
