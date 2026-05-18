use serde_json::Value;
use sqlx::Row;

#[derive(Clone, Debug)]
pub struct StrategyObservation {
    pub event_source: String,
    pub payload: Value,
    pub replay_block: Option<i64>,
}

pub async fn query_strategy_observations(
    pool: &sqlx::PgPool,
    replay_run_id: &str,
    from_block: Option<u64>,
    to_block: Option<u64>,
) -> sqlx::Result<Vec<StrategyObservation>> {
    let replay_block_expr =
        "COALESCE(block_number, NULLIF(payload->>'live_current_block', '')::BIGINT)";
    let mut query = format!(
        "SELECT event_source, payload, {replay_block_expr} AS replay_block
         FROM alpha_trading.strategy_observations
         WHERE run_id = $1",
    );
    if from_block.is_some() {
        query.push_str(&format!(" AND {replay_block_expr} >= $2"));
    }
    if to_block.is_some() {
        query.push_str(&format!(
            " AND {replay_block_expr} <= ${}",
            if from_block.is_some() { 3 } else { 2 }
        ));
    }
    query.push_str(&format!(
        " ORDER BY {replay_block_expr} ASC NULLS LAST, first_seen_at ASC"
    ));

    let mut q = sqlx::query(&query).bind(replay_run_id);
    if let Some(block) = from_block {
        q = q.bind(block as i64);
    }
    if let Some(block) = to_block {
        q = q.bind(block as i64);
    }

    let rows = q.fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| {
            Ok(StrategyObservation {
                event_source: row.try_get("event_source")?,
                payload: row.try_get("payload")?,
                replay_block: row.try_get("replay_block")?,
            })
        })
        .collect()
}
