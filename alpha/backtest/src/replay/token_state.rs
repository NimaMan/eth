use std::str::FromStr;

use alloy_primitives::Address;
use eth_alpha_core::{
    ids::TokenPoolId,
    risk::{RiskEvent, RiskKind, RiskSeverity, RISK_SOURCE_POOL_UPDATE},
};
use eyre::{Result, WrapErr};
use serde_json::{json, Map, Value};

#[derive(Debug, sqlx::FromRow)]
struct PoolLatestRiskRow {
    token_address: String,
    pool_id: String,
    protocol: Option<String>,
    lifecycle_status: Option<String>,
    valuation_status: String,
    liquidity_removal_block: Option<i64>,
    liquidity_removal_tx: Option<String>,
    liquidity_removal_label: Option<String>,
}

/// Load mined terminal pool/custody events from token_state.pool_latest.
///
/// This overlays the replay with the same mined pool-state risk signal that live
/// trading uses, including custody-class buyer-token confiscation labels that do
/// not necessarily have a reserve-removal event in the pool update stream.
pub async fn load_events_from_token_state(
    pool: &sqlx::PgPool,
    scope_id: &str,
    from_block: Option<u64>,
    to_block: Option<u64>,
    allowed_protocols: &[String],
) -> Result<Vec<eth_alpha_engine::EngineEvent>> {
    let from_block = from_block.and_then(|block| i64::try_from(block).ok());
    let to_block = to_block.and_then(|block| i64::try_from(block).ok());
    let allowed_protocols = normalized_allowed_protocols(allowed_protocols);
    let include_all_protocols = allowed_protocols.is_empty();

    let rows = sqlx::query_as::<_, PoolLatestRiskRow>(
        r#"
        SELECT
            token_address,
            pool_id,
            protocol,
            lifecycle_status,
            valuation_status,
            liquidity_removal_block,
            liquidity_removal_tx,
            liquidity_removal_label
        FROM token_state.pool_latest
        WHERE scope_id = $1
          AND chain_id = 1
          AND has_liquidity_removal = true
          AND liquidity_removal_block IS NOT NULL
          AND ($2::bigint IS NULL OR liquidity_removal_block >= $2)
          AND ($3::bigint IS NULL OR liquidity_removal_block <= $3)
          AND ($4::boolean OR upper(coalesce(protocol, '')) = ANY($5::text[]))
        ORDER BY liquidity_removal_block, token_address, pool_id
        "#,
    )
    .bind(scope_id)
    .bind(from_block)
    .bind(to_block)
    .bind(include_all_protocols)
    .bind(&allowed_protocols)
    .fetch_all(pool)
    .await
    .wrap_err("failed to query token_state.pool_latest terminal pool events")?;

    let mut events = Vec::with_capacity(rows.len());
    let mut skipped = 0usize;
    for row in rows {
        let Some(block) = positive_block(row.liquidity_removal_block) else {
            skipped += 1;
            continue;
        };
        let token_address = match Address::from_str(row.token_address.trim()) {
            Ok(address) => address,
            Err(error) => {
                tracing::warn!(
                    token_address = %row.token_address,
                    error = %error,
                    "skipping token_state terminal event with invalid token address"
                );
                skipped += 1;
                continue;
            }
        };
        let pool_address = TokenPoolId::new(token_address, &row.pool_id);
        let risk_kind_tag = terminal_risk_kind_tag(row.liquidity_removal_label.as_deref());
        let message =
            terminal_risk_message(risk_kind_tag, block, row.liquidity_removal_label.as_deref());
        let evidence = terminal_risk_evidence(scope_id, &row, &pool_address, risk_kind_tag, block);

        events.push(eth_alpha_engine::EngineEvent::Risk(RiskEvent {
            kind: RiskKind::LiquidityRemoval,
            severity: RiskSeverity::Critical,
            source: Some(RISK_SOURCE_POOL_UPDATE.to_string()),
            token_address,
            pool_address: Some(pool_address),
            pending_tx_hash: None,
            observed_block: Some(block),
            message,
            evidence: Some(Value::Object(evidence)),
        }));
    }

    tracing::info!(
        scope_id,
        allowed_protocols = ?allowed_protocols,
        events = events.len(),
        skipped,
        "loaded token_state terminal pool risk overlay"
    );

    Ok(events)
}

fn normalized_allowed_protocols(protocols: &[String]) -> Vec<String> {
    protocols
        .iter()
        .map(|protocol| protocol.trim().to_ascii_uppercase())
        .filter(|protocol| !protocol.is_empty())
        .collect()
}

fn positive_block(block: Option<i64>) -> Option<u64> {
    block
        .and_then(|block| u64::try_from(block).ok())
        .filter(|block| *block > 0)
}

fn terminal_risk_kind_tag(label: Option<&str>) -> &'static str {
    let label = label.map(str::to_ascii_lowercase).unwrap_or_default();
    if label.contains("confiscation") || label.contains("custody") {
        "custody_buyer_token_confiscation"
    } else if label.contains("holder-balance") || label.contains("holder_balance") {
        "holder_balance_backdoor_drain"
    } else {
        "liquidity_removal"
    }
}

fn terminal_risk_message(risk_kind_tag: &str, block: u64, label: Option<&str>) -> String {
    match risk_kind_tag {
        "custody_buyer_token_confiscation" => {
            format!("backdoor buyer-token confiscation rug at block {block}")
        }
        "holder_balance_backdoor_drain" => {
            format!("holder-balance backdoor drain at block {block}")
        }
        _ => format!(
            "token-state mined terminal pool event at block {block}: {}",
            label.unwrap_or("liquidity removal")
        ),
    }
}

fn terminal_risk_evidence(
    scope_id: &str,
    row: &PoolLatestRiskRow,
    pool_address: &TokenPoolId,
    risk_kind_tag: &str,
    observed_block: u64,
) -> Map<String, Value> {
    let mut evidence = Map::new();
    evidence.insert("source".to_string(), json!("token_state"));
    evidence.insert("scope_id".to_string(), json!(scope_id));
    evidence.insert("risk_kind".to_string(), json!(risk_kind_tag));
    evidence.insert("observed_block".to_string(), json!(observed_block));
    evidence.insert("token_address".to_string(), json!(row.token_address));
    evidence.insert("pool_address".to_string(), json!(pool_address.to_string()));
    evidence.insert("token_state_pool_id".to_string(), json!(row.pool_id));
    evidence.insert("protocol".to_string(), json!(row.protocol));
    evidence.insert("lifecycle_status".to_string(), json!(row.lifecycle_status));
    evidence.insert("valuation_status".to_string(), json!(row.valuation_status));
    evidence.insert("liquidity_removal".to_string(), json!(true));
    evidence.insert("liquidity_removal_block".to_string(), json!(observed_block));
    evidence.insert(
        "liquidity_removal_tx_hash".to_string(),
        json!(row.liquidity_removal_tx),
    );
    evidence.insert(
        "liquidity_removal_label".to_string(),
        json!(row.liquidity_removal_label),
    );
    evidence
}
