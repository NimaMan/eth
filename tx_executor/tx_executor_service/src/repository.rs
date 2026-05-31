use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{postgres::PgRow, Row};
use sqlx::{types::Json, PgPool};
use tx_executor::DirectRawTransactionRequest;

use crate::policy::{metadata_string, EthTxPolicyEvaluation};
use crate::{execution_config::ExecutionConfig, postgres};

#[derive(Clone)]
pub struct EthTxPolicyRepository {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct EthTxDailySpendSnapshot {
    pub spend_day: String,
    pub spent_wei: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EthTxPolicyDecisionRecord {
    pub id: i64,
    pub attempt_id: String,
    pub created_at: String,
    pub route: String,
    pub decision: String,
    pub policy_version: String,
    pub reasons: Value,
    pub request: Value,
    pub normalized: Value,
    pub metadata: Value,
    pub strategy_name: Option<String>,
    pub strategy_run_id: Option<String>,
    pub trade_id: Option<String>,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub selector: Option<String>,
    pub value_wei: Option<String>,
    pub gas_limit: Option<String>,
    pub max_fee_per_gas_wei: Option<String>,
    pub max_priority_fee_per_gas_wei: Option<String>,
    pub estimated_worst_case_cost_wei: Option<String>,
    pub simulation_block_number: Option<i64>,
    pub latest_block_number: Option<i64>,
}

impl EthTxPolicyRepository {
    pub async fn connect(config: &ExecutionConfig) -> Result<Self> {
        Ok(Self {
            pool: postgres::connect_pool(config).await?,
        })
    }

    pub async fn daily_spend_snapshot(&self) -> Result<EthTxDailySpendSnapshot> {
        let spend_day = Utc::now().date_naive();
        let spent_wei = sqlx::query_as::<_, (String,)>(
            r#"
            SELECT spent_wei
            FROM eth_tx_execution.eth_tx_policy_daily_spend
            WHERE spend_day = $1
            "#,
        )
        .bind(spend_day)
        .fetch_optional(&self.pool)
        .await
        .context("failed to fetch ETH tx daily spend")?
        .map(|row| row.0)
        .unwrap_or_else(|| "0".to_string());

        Ok(EthTxDailySpendSnapshot {
            spend_day: spend_day.to_string(),
            spent_wei,
        })
    }

    pub async fn list_decisions(&self, limit: i64) -> Result<Vec<EthTxPolicyDecisionRecord>> {
        let rows = sqlx::query(POLICY_DECISION_SELECT_RECENT)
            .bind(normalize_limit(limit))
            .fetch_all(&self.pool)
            .await
            .context("failed to list ETH tx policy decisions")?;
        rows.into_iter().map(policy_decision_from_row).collect()
    }

    pub async fn list_decisions_for_attempt(
        &self,
        attempt_id: &str,
        limit: i64,
    ) -> Result<Vec<EthTxPolicyDecisionRecord>> {
        let rows = sqlx::query(POLICY_DECISION_SELECT_BY_ATTEMPT)
            .bind(attempt_id)
            .bind(normalize_limit(limit))
            .fetch_all(&self.pool)
            .await
            .context("failed to list ETH tx policy decisions for attempt")?;
        rows.into_iter().map(policy_decision_from_row).collect()
    }

    pub async fn record_evaluation(
        &self,
        evaluation: EthTxPolicyEvaluation,
        request: &DirectRawTransactionRequest,
        max_daily_cost_wei: u128,
        route: &str,
    ) -> Result<EthTxPolicyEvaluation> {
        if !evaluation.is_accepted() {
            self.insert_decision(&evaluation, request, route).await?;
            return Ok(evaluation);
        }

        let Some(cost_wei) = evaluation
            .normalized
            .estimated_worst_case_cost_wei
            .as_deref()
            .and_then(|value| value.parse::<u128>().ok())
        else {
            let rejected = evaluation.with_rejection("policy could not compute spend cost");
            self.insert_decision(&rejected, request, route).await?;
            return Ok(rejected);
        };

        if max_daily_cost_wei == 0 {
            self.insert_decision(&evaluation, request, route).await?;
            return Ok(evaluation);
        }

        let spend_day = Utc::now().date_naive();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start ETH tx policy transaction")?;

        sqlx::query(
            r#"
            INSERT INTO eth_tx_execution.eth_tx_policy_daily_spend (spend_day, spent_wei, updated_at)
            VALUES ($1, '0', now())
            ON CONFLICT (spend_day) DO NOTHING
            "#,
        )
        .bind(spend_day)
        .execute(&mut *tx)
        .await
        .context("failed to initialize ETH tx daily spend row")?;

        let (spent_wei_text,): (String,) = sqlx::query_as(
            r#"
            SELECT spent_wei
            FROM eth_tx_execution.eth_tx_policy_daily_spend
            WHERE spend_day = $1
            FOR UPDATE
            "#,
        )
        .bind(spend_day)
        .fetch_one(&mut *tx)
        .await
        .context("failed to lock ETH tx daily spend row")?;
        let spent_wei = spent_wei_text
            .parse::<u128>()
            .context("invalid stored ETH tx daily spend")?;

        let existing_reservation: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT cost_wei
            FROM eth_tx_execution.eth_tx_spend_reservations
            WHERE attempt_id = $1
            "#,
        )
        .bind(&evaluation.attempt_id)
        .fetch_optional(&mut *tx)
        .await
        .context("failed to check ETH tx spend reservation")?;

        let needs_new_reservation = existing_reservation.is_none();
        if needs_new_reservation {
            let next_spend = spent_wei
                .checked_add(cost_wei)
                .context("ETH tx daily spend overflow")?;
            if next_spend > max_daily_cost_wei {
                let rejected = evaluation.with_rejection(format!(
                    "daily spend {} plus cost {} exceeds policy cap {}",
                    spent_wei, cost_wei, max_daily_cost_wei
                ));
                insert_decision_in_tx(&mut tx, &rejected, request, route).await?;
                tx.commit()
                    .await
                    .context("failed to commit rejected ETH tx policy decision")?;
                return Ok(rejected);
            }

            sqlx::query(
                r#"
                INSERT INTO eth_tx_execution.eth_tx_spend_reservations (
                    attempt_id, spend_day, cost_wei, created_at
                )
                VALUES ($1, $2, $3, now())
                "#,
            )
            .bind(&evaluation.attempt_id)
            .bind(spend_day)
            .bind(cost_wei.to_string())
            .execute(&mut *tx)
            .await
            .context("failed to reserve ETH tx policy spend")?;

            sqlx::query(
                r#"
                UPDATE eth_tx_execution.eth_tx_policy_daily_spend
                SET spent_wei = $2, updated_at = now()
                WHERE spend_day = $1
                "#,
            )
            .bind(spend_day)
            .bind(next_spend.to_string())
            .execute(&mut *tx)
            .await
            .context("failed to update ETH tx daily spend")?;
        }

        insert_decision_in_tx(&mut tx, &evaluation, request, route).await?;
        tx.commit()
            .await
            .context("failed to commit accepted ETH tx policy decision")?;
        Ok(evaluation)
    }

    pub async fn release_spend_reservation(&self, attempt_id: &str) -> Result<Option<String>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start ETH tx spend release transaction")?;

        let reservation: Option<(chrono::NaiveDate, String)> = sqlx::query_as(
            r#"
            SELECT spend_day, cost_wei
            FROM eth_tx_execution.eth_tx_spend_reservations
            WHERE attempt_id = $1
            FOR UPDATE
            "#,
        )
        .bind(attempt_id)
        .fetch_optional(&mut *tx)
        .await
        .context("failed to fetch ETH tx spend reservation for release")?;

        let Some((spend_day, cost_wei_text)) = reservation else {
            tx.commit()
                .await
                .context("failed to commit empty ETH tx spend release")?;
            return Ok(None);
        };

        let cost_wei = cost_wei_text
            .parse::<u128>()
            .context("invalid ETH tx spend reservation cost")?;
        let (spent_wei_text,): (String,) = sqlx::query_as(
            r#"
            SELECT spent_wei
            FROM eth_tx_execution.eth_tx_policy_daily_spend
            WHERE spend_day = $1
            FOR UPDATE
            "#,
        )
        .bind(spend_day)
        .fetch_one(&mut *tx)
        .await
        .context("failed to lock ETH tx daily spend row for release")?;
        let spent_wei = spent_wei_text
            .parse::<u128>()
            .context("invalid stored ETH tx daily spend")?;
        let next_spend = daily_spend_after_release(spent_wei, cost_wei);

        sqlx::query(
            r#"
            UPDATE eth_tx_execution.eth_tx_policy_daily_spend
            SET spent_wei = $2, updated_at = now()
            WHERE spend_day = $1
            "#,
        )
        .bind(spend_day)
        .bind(next_spend.to_string())
        .execute(&mut *tx)
        .await
        .context("failed to update ETH tx daily spend during release")?;

        sqlx::query(
            r#"
            DELETE FROM eth_tx_execution.eth_tx_spend_reservations
            WHERE attempt_id = $1
            "#,
        )
        .bind(attempt_id)
        .execute(&mut *tx)
        .await
        .context("failed to delete ETH tx spend reservation")?;

        tx.commit()
            .await
            .context("failed to commit ETH tx spend release")?;
        Ok(Some(cost_wei_text))
    }

    async fn insert_decision(
        &self,
        evaluation: &EthTxPolicyEvaluation,
        request: &DirectRawTransactionRequest,
        route: &str,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start ETH tx policy decision transaction")?;
        insert_decision_in_tx(&mut tx, evaluation, request, route).await?;
        tx.commit()
            .await
            .context("failed to commit ETH tx policy decision")?;
        Ok(())
    }
}

const POLICY_DECISION_SELECT_RECENT: &str = r#"
    SELECT
        id, attempt_id, created_at, route, decision, policy_version, reasons_json,
        request_json, normalized_json, metadata_json,
        strategy_name, strategy_run_id, trade_id, token_address, pool_address,
        from_address, to_address, selector, value_wei, gas_limit,
        max_fee_per_gas_wei, max_priority_fee_per_gas_wei,
        estimated_worst_case_cost_wei, simulation_block_number, latest_block_number
    FROM eth_tx_execution.eth_tx_policy_decisions
    ORDER BY created_at DESC, id DESC
    LIMIT $1
"#;

const POLICY_DECISION_SELECT_BY_ATTEMPT: &str = r#"
    SELECT
        id, attempt_id, created_at, route, decision, policy_version, reasons_json,
        request_json, normalized_json, metadata_json,
        strategy_name, strategy_run_id, trade_id, token_address, pool_address,
        from_address, to_address, selector, value_wei, gas_limit,
        max_fee_per_gas_wei, max_priority_fee_per_gas_wei,
        estimated_worst_case_cost_wei, simulation_block_number, latest_block_number
    FROM eth_tx_execution.eth_tx_policy_decisions
    WHERE attempt_id = $1
    ORDER BY created_at DESC, id DESC
    LIMIT $2
"#;

fn normalize_limit(limit: i64) -> i64 {
    limit.clamp(1, 500)
}

fn policy_decision_from_row(row: PgRow) -> Result<EthTxPolicyDecisionRecord> {
    let created_at: DateTime<Utc> = row.try_get("created_at")?;

    Ok(EthTxPolicyDecisionRecord {
        id: row.try_get("id")?,
        attempt_id: row.try_get("attempt_id")?,
        created_at: created_at.to_rfc3339(),
        route: row.try_get("route")?,
        decision: row.try_get("decision")?,
        policy_version: row.try_get("policy_version")?,
        reasons: row.try_get("reasons_json")?,
        request: row.try_get("request_json")?,
        normalized: row.try_get("normalized_json")?,
        metadata: row.try_get("metadata_json")?,
        strategy_name: row.try_get("strategy_name")?,
        strategy_run_id: row.try_get("strategy_run_id")?,
        trade_id: row.try_get("trade_id")?,
        token_address: row.try_get("token_address")?,
        pool_address: row.try_get("pool_address")?,
        from_address: row.try_get("from_address")?,
        to_address: row.try_get("to_address")?,
        selector: row.try_get("selector")?,
        value_wei: row.try_get("value_wei")?,
        gas_limit: row.try_get("gas_limit")?,
        max_fee_per_gas_wei: row.try_get("max_fee_per_gas_wei")?,
        max_priority_fee_per_gas_wei: row.try_get("max_priority_fee_per_gas_wei")?,
        estimated_worst_case_cost_wei: row.try_get("estimated_worst_case_cost_wei")?,
        simulation_block_number: row.try_get("simulation_block_number")?,
        latest_block_number: row.try_get("latest_block_number")?,
    })
}

async fn insert_decision_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    evaluation: &EthTxPolicyEvaluation,
    request: &DirectRawTransactionRequest,
    route: &str,
) -> Result<()> {
    let request_json =
        serde_json::to_value(request).context("failed to serialize ETH tx request")?;
    let metadata_json = if request.metadata.is_object() {
        request.metadata.clone()
    } else {
        json!({})
    };
    let normalized = &evaluation.normalized;

    sqlx::query(
        r#"
        INSERT INTO eth_tx_execution.eth_tx_policy_decisions (
            attempt_id, created_at, route, decision, policy_version, reasons_json,
            request_json, normalized_json, metadata_json,
            strategy_name, strategy_run_id, trade_id, token_address, pool_address,
            from_address, to_address, selector, value_wei, gas_limit,
            max_fee_per_gas_wei, max_priority_fee_per_gas_wei,
            estimated_worst_case_cost_wei, simulation_block_number, latest_block_number
        )
        VALUES (
            $1, now(), $2, $3, $4, $5,
            $6, $7, $8,
            $9, $10, $11, $12, $13,
            $14, $15, $16, $17, $18,
            $19, $20,
            $21, $22, $23
        )
        "#,
    )
    .bind(&evaluation.attempt_id)
    .bind(route)
    .bind(evaluation.outcome.as_str())
    .bind(&evaluation.policy_version)
    .bind(Json(json!(evaluation.reasons)))
    .bind(Json(request_json))
    .bind(Json(evaluation.normalized.to_json()))
    .bind(Json(metadata_json))
    .bind(metadata_string(&request.metadata, "strategy_name"))
    .bind(metadata_string(&request.metadata, "strategy_run_id"))
    .bind(metadata_string(&request.metadata, "trade_id"))
    .bind(metadata_string(&request.metadata, "token_address"))
    .bind(metadata_string(&request.metadata, "pool_address"))
    .bind(&normalized.from)
    .bind(&normalized.to)
    .bind(&normalized.selector)
    .bind(&normalized.value_wei)
    .bind(&normalized.gas_limit)
    .bind(&normalized.max_fee_per_gas_wei)
    .bind(&normalized.max_priority_fee_per_gas_wei)
    .bind(&normalized.estimated_worst_case_cost_wei)
    .bind(normalized.simulation_block_number.map(|value| value as i64))
    .bind(normalized.latest_block_number.map(|value| value as i64))
    .execute(&mut **tx)
    .await
    .context("failed to insert ETH tx policy decision")?;

    Ok(())
}

pub fn policy_decision_json(evaluation: &EthTxPolicyEvaluation) -> Value {
    json!({
        "attempt_id": evaluation.attempt_id,
        "decision": evaluation.outcome.as_str(),
        "policy_version": evaluation.policy_version,
        "reasons": evaluation.reasons,
        "normalized": evaluation.normalized,
    })
}

fn daily_spend_after_release(spent_wei: u128, reserved_cost_wei: u128) -> u128 {
    spent_wei.saturating_sub(reserved_cost_wei)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_subtracts_reserved_cost_from_daily_spend() {
        assert_eq!(daily_spend_after_release(25, 10), 15);
    }

    #[test]
    fn release_never_underflows_daily_spend() {
        assert_eq!(daily_spend_after_release(10, 25), 0);
    }
}
