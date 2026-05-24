use eth_alpha_core::{decision_rationale::DecisionReason, risk::RiskEvent};
use serde_json::{json, Map, Value};

pub(crate) fn enrich_decision_reason_with_risk_event(
    reason: &mut DecisionReason,
    event: &RiskEvent,
) {
    if let Some(details) = enrich_reason_details_value(reason.details.take(), event) {
        reason.details = details;
    }
}

pub(crate) fn enrich_reason_details_with_risk_event(
    reason_details: &mut Option<Value>,
    event: &RiskEvent,
) {
    let current = reason_details.take().unwrap_or(Value::Null);
    if let Some(details) = enrich_reason_details_value(current, event) {
        *reason_details = Some(details);
    }
}

pub(crate) fn attach_risk_event_evidence_to_payload(payload: &mut Value, event: &RiskEvent) {
    let Some(evidence) = event.evidence.as_ref() else {
        return;
    };
    if !payload.is_object() {
        *payload = json!({});
    }
    if let Some(map) = payload.as_object_mut() {
        map.insert("risk_event_evidence".to_string(), evidence.clone());
    }
}

fn enrich_reason_details_value(details: Value, event: &RiskEvent) -> Option<Value> {
    let mut details = match details {
        Value::Object(map) => map,
        Value::Null => Map::new(),
        _ => Map::new(),
    };
    if let Some(pending_tx_hash) = event.pending_tx_hash {
        details.insert(
            "pending_tx_hash".to_string(),
            json!(pending_tx_hash.to_string()),
        );
    }
    if let Some(observed_block) = event.observed_block {
        details.insert("risk_observed_block".to_string(), json!(observed_block));
    }
    if let Some(evidence) = event.evidence.as_ref() {
        details.insert("risk_event_evidence".to_string(), evidence.clone());
        copy_evidence_field(&mut details, evidence, "lp_approval_age_basis", "age_basis");
        copy_evidence_field(
            &mut details,
            evidence,
            "trading_enabled_age_blocks_at_signal",
            "trading_enabled_age_blocks",
        );
        copy_evidence_field(
            &mut details,
            evidence,
            "pool_age_blocks_at_signal",
            "pool_age_blocks",
        );
        copy_evidence_field(
            &mut details,
            evidence,
            "trading_enabled_block",
            "trading_enabled_block",
        );
        copy_evidence_field(
            &mut details,
            evidence,
            "pool_creation_block",
            "pool_creation_block",
        );
        copy_evidence_field(
            &mut details,
            evidence,
            "observed_block",
            "risk_observed_block",
        );
        copy_evidence_field(&mut details, evidence, "signal_id", "signal_id");
        copy_evidence_field(
            &mut details,
            evidence,
            "mempool_first_seen_at",
            "mempool_first_seen_at",
        );
    }
    if details.is_empty() {
        None
    } else {
        Some(Value::Object(details))
    }
}

fn copy_evidence_field(
    target: &mut Map<String, Value>,
    evidence: &Value,
    source_key: &str,
    target_key: &str,
) {
    if let Some(value) = evidence.get(source_key) {
        target.insert(target_key.to_string(), value.clone());
    }
}
