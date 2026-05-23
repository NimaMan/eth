use eyre::Result;
use serde_json::to_value;
use sqlx::PgPool;

use crate::api::RiskAtlasPageView;
use crate::ingest::report::RiskAtlasReportImport;

use super::schema::{EventEvidenceRow, ObservationRow, PoolEligibilityRow, RiskAtlasRun};

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

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn replace_report_import(&self, import: &RiskAtlasReportImport) -> Result<()> {
        self.upsert_run(&import.run).await?;

        let mut tx = self.pool.begin().await?;
        for table in [
            "risk_atlas_distributions",
            "risk_atlas_pool_eligibility",
            "risk_atlas_event_evidence",
            "risk_atlas_observations",
            "risk_atlas_numeric_stats",
            "risk_atlas_active_targets",
            "risk_atlas_decision_questions",
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

        for item in &import.event_evidence {
            insert_event_evidence_row(&mut tx, &import.run.run_id, item).await?;
        }

        for item in &import.observations {
            insert_observation_row(&mut tx, &import.run.run_id, item).await?;
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
            let target_key = active_target_key(&item.row_kind, item.active_observation_delta);
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
            .bind(item.active_observation_delta)
            .bind(item.rows)
            .bind(item.unique_pools)
            .bind(item.positives)
            .bind(item.negatives)
            .bind(item.sort_order)
            .execute(&mut *tx)
            .await?;
        }

        for item in &import.decision_questions {
            sqlx::query(
                r#"
                INSERT INTO risk_atlas_decision_questions (
                    run_id, question_id, category, question, headline, answer, status,
                    denominator_label, denominator_count, payload, sort_order
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
            )
            .bind(&import.run.run_id)
            .bind(&item.question_id)
            .bind(&item.category)
            .bind(&item.question)
            .bind(&item.headline)
            .bind(&item.answer)
            .bind(&item.status)
            .bind(&item.denominator_label)
            .bind(item.denominator_count)
            .bind(&item.payload)
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
            .bind(item.trading_enabled_to_label_chain_block_delta)
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

async fn insert_event_evidence_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: &str,
    item: &EventEvidenceRow,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO risk_atlas_event_evidence (
            run_id, token_address, pool_address, protocol, event_kind, mechanism,
            block_number, block_timestamp, tx_hash, mempool_first_seen_ms, source, sort_order
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(run_id)
    .bind(&item.token_address)
    .bind(&item.pool_address)
    .bind(&item.protocol)
    .bind(&item.event_kind)
    .bind(&item.mechanism)
    .bind(item.block_number)
    .bind(item.block_timestamp)
    .bind(&item.tx_hash)
    .bind(item.mempool_first_seen_ms)
    .bind(&item.source)
    .bind(item.sort_order)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn insert_observation_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: &str,
    item: &ObservationRow,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO risk_atlas_observations (
            run_id, token_address, pool_address, denom_address, protocol,
            active_observation_index, block_number, timestamp, active_reasons,
            tx_count, token_transfer_count, denom_transfer_count,
            buy_volume_denom, sell_volume_denom, total_bribe_eth,
            can_buy, can_sell, effective_can_buy, effective_can_sell,
            buy_tax, sell_tax, liquidity_removed_as_of, liquidity_removal_in_block,
            liquidity_removal_block_as_of, direct_lp_removal_as_of, direct_lp_removal_in_block,
            direct_lp_target_1, direct_lp_target_2, direct_lp_target_3, direct_lp_target_5,
            direct_lp_target_10, denom_reserve, token_reserve, total_liquidity_denom,
            price_to_initial_ratio, lp_approved_pct_as_of,
            token_decimals, price_denom_per_token, initial_price_denom_per_token,
            lp_approval_count_in_block, lp_approval_seen_as_of,
            lp_total_supply, lp_max_approval_amount_as_of, lp_max_approval_pct_as_of,
            lp_removable_pct_as_of, lp_router_removable_pct_as_of, lp_approval_owner_is_creator,
            creator_lp_balance_pct_as_of, creator_lp_approved_pct_as_of,
            creator_lp_removable_pct_as_of, creator_lp_router_removable_pct_as_of,
            creator_lp_approved_gt_90_pct_as_of, creator_lp_router_removable_gt_90_pct_as_of,
            last_lp_approval_amount_pct_of_total_supply,
            first_lp_approval_to_as_of_chain_block_delta,
            last_lp_approval_to_as_of_chain_block_delta,
            pool_creation_to_first_lp_approval_chain_block_delta,
            pool_creation_to_last_lp_approval_chain_block_delta,
            trading_enabled_to_first_lp_approval_chain_block_delta,
            trading_enabled_to_last_lp_approval_chain_block_delta,
            control_transfer_from_after_renounce_seen_as_of,
            control_transfer_from_after_renounce_in_block,
            control_transfer_from_holder_to_burn_seen_as_of,
            control_transfer_from_holder_to_burn_in_block,
            control_transfer_from_pair_seen_as_of,
            control_transfer_from_pair_in_block,
            control_transfer_from_without_transfer_log_seen_as_of,
            control_transfer_from_without_transfer_log_in_block,
            pair_token_to_control_seen_as_of,
            pair_token_to_control_in_block,
            pair_token_to_control_to_pool_reserve_ratio,
            pair_balance_backdoor_signal_seen_as_of,
            pair_balance_backdoor_signal_in_block,
            last_pair_balance_backdoor_signal_to_as_of_chain_block_delta,
            token_transfer_to_total_supply_ratio, token_transfer_to_pool_token_reserve_ratio,
            observation, features
        )
        VALUES (
            $1, $2, $3, $4, $5,
            $6, $7, $8, $9,
            $10, $11, $12,
            $13, $14, $15,
            $16, $17, $18, $19,
            $20, $21, $22, $23,
            $24, $25, $26,
            $27, $28, $29, $30,
            $31, $32, $33, $34,
            $35, $36,
            $37, $38, $39,
            $40, $41,
            $42, $43, $44,
            $45, $46, $47,
            $48, $49,
            $50, $51,
            $52, $53,
            $54,
            $55,
            $56,
            $57,
            $58,
            $59,
            $60,
            $61,
            $62,
            $63,
            $64,
            $65,
            $66,
            $67,
            $68,
            $69,
            $70,
            $71,
            $72,
            $73,
            $74,
            $75, $76,
            $77, $78
        )
        "#,
    )
    .bind(run_id)
    .bind(&item.token_address)
    .bind(&item.pool_address)
    .bind(&item.denom_address)
    .bind(&item.protocol)
    .bind(item.active_observation_index)
    .bind(item.block_number)
    .bind(item.timestamp)
    .bind(&item.active_reasons)
    .bind(item.tx_count)
    .bind(item.token_transfer_count)
    .bind(item.denom_transfer_count)
    .bind(item.buy_volume_denom)
    .bind(item.sell_volume_denom)
    .bind(item.total_bribe_eth)
    .bind(item.can_buy)
    .bind(item.can_sell)
    .bind(item.effective_can_buy)
    .bind(item.effective_can_sell)
    .bind(item.buy_tax)
    .bind(item.sell_tax)
    .bind(item.liquidity_removed_as_of)
    .bind(item.liquidity_removal_in_block)
    .bind(item.liquidity_removal_block_as_of)
    .bind(item.direct_lp_removal_as_of)
    .bind(item.direct_lp_removal_in_block)
    .bind(item.direct_lp_target_1)
    .bind(item.direct_lp_target_2)
    .bind(item.direct_lp_target_3)
    .bind(item.direct_lp_target_5)
    .bind(item.direct_lp_target_10)
    .bind(item.denom_reserve)
    .bind(item.token_reserve)
    .bind(item.total_liquidity_denom)
    .bind(item.price_to_initial_ratio)
    .bind(item.lp_approved_pct_as_of)
    .bind(item.token_decimals)
    .bind(item.price_denom_per_token)
    .bind(item.initial_price_denom_per_token)
    .bind(item.lp_approval_count_in_block)
    .bind(item.lp_approval_seen_as_of)
    .bind(item.lp_total_supply)
    .bind(item.lp_max_approval_amount_as_of)
    .bind(item.lp_max_approval_pct_as_of)
    .bind(item.lp_removable_pct_as_of)
    .bind(item.lp_router_removable_pct_as_of)
    .bind(item.lp_approval_owner_is_creator)
    .bind(item.creator_lp_balance_pct_as_of)
    .bind(item.creator_lp_approved_pct_as_of)
    .bind(item.creator_lp_removable_pct_as_of)
    .bind(item.creator_lp_router_removable_pct_as_of)
    .bind(item.creator_lp_approved_gt_90_pct_as_of)
    .bind(item.creator_lp_router_removable_gt_90_pct_as_of)
    .bind(item.last_lp_approval_amount_pct_of_total_supply)
    .bind(item.first_lp_approval_to_as_of_chain_block_delta)
    .bind(item.last_lp_approval_to_as_of_chain_block_delta)
    .bind(item.pool_creation_to_first_lp_approval_chain_block_delta)
    .bind(item.pool_creation_to_last_lp_approval_chain_block_delta)
    .bind(item.trading_enabled_to_first_lp_approval_chain_block_delta)
    .bind(item.trading_enabled_to_last_lp_approval_chain_block_delta)
    .bind(item.control_transfer_from_after_renounce_seen_as_of)
    .bind(item.control_transfer_from_after_renounce_in_block)
    .bind(item.control_transfer_from_holder_to_burn_seen_as_of)
    .bind(item.control_transfer_from_holder_to_burn_in_block)
    .bind(item.control_transfer_from_pair_seen_as_of)
    .bind(item.control_transfer_from_pair_in_block)
    .bind(item.control_transfer_from_without_transfer_log_seen_as_of)
    .bind(item.control_transfer_from_without_transfer_log_in_block)
    .bind(item.pair_token_to_control_seen_as_of)
    .bind(item.pair_token_to_control_in_block)
    .bind(item.pair_token_to_control_to_pool_reserve_ratio)
    .bind(item.pair_balance_backdoor_signal_seen_as_of)
    .bind(item.pair_balance_backdoor_signal_in_block)
    .bind(item.last_pair_balance_backdoor_signal_to_as_of_chain_block_delta)
    .bind(item.token_transfer_to_total_supply_ratio)
    .bind(item.token_transfer_to_pool_token_reserve_ratio)
    .bind(&item.observation)
    .bind(&item.features)
    .execute(&mut **tx)
    .await?;

    Ok(())
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
