use std::collections::HashMap;
use std::sync::{atomic::Ordering, Arc};

use eth_alpha_core::{
    ids::PoolAddress,
    market::PoolSnapshot,
    mempool_entry::projected_pool_from_risk_event,
    risk::{RiskEvent, RiskKind},
};
use serde_json::{json, Map, Value};

use eth_alpha_engine::wire::{MempoolSignalWire, PoolWire};

pub(super) fn prime_projected_mempool_entry_pool(
    event: &RiskEvent,
    pool_updates: &Arc<std::sync::Mutex<HashMap<PoolAddress, PoolSnapshot>>>,
    adapter_current_block: &Arc<std::sync::atomic::AtomicU64>,
) -> Option<Value> {
    let pool = projected_pool_from_risk_event(event)?;
    let pool_address = pool.address.clone();
    let token_address = pool.token_address;
    let latest_block = pool.latest_block;
    let can_buy = pool.can_buy;
    let can_sell = pool.can_sell;
    pool_updates
        .lock()
        .expect("pool lock")
        .insert(pool_address.clone(), pool);
    let current_block = adapter_current_block.load(Ordering::Relaxed);
    if latest_block > current_block {
        adapter_current_block.store(latest_block, Ordering::Relaxed);
    }
    Some(json!({
        "source": "mempool_entry_evidence",
        "pool_address": pool_address.0,
        "token_address": token_address.to_string(),
        "latest_block": latest_block,
        "can_buy": can_buy,
        "can_sell": can_sell,
    }))
}

pub(super) fn annotate_signal_risk_event(
    event: &mut RiskEvent,
    signal: &MempoolSignalWire,
    pool: Option<&PoolWire>,
) {
    let mut evidence = event
        .evidence
        .take()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    evidence.insert("signal_id".to_string(), json!(signal.signal_id));
    evidence.insert("signal_type".to_string(), json!(signal.signal_type));
    if let Some(value) = signal.signal_source.as_ref() {
        evidence.insert("signal_source".to_string(), json!(value));
    }
    if let Some(value) = event.observed_block {
        evidence.insert("observed_block".to_string(), json!(value));
    }
    if event.kind == RiskKind::LpApproval {
        annotate_lp_approval_age_evidence(&mut evidence, event, pool);
    }
    event.evidence = Some(Value::Object(evidence));
}

fn annotate_lp_approval_age_evidence(
    evidence: &mut Map<String, Value>,
    event: &RiskEvent,
    pool: Option<&PoolWire>,
) {
    let Some(observed_block) = event.observed_block else {
        evidence.insert("lp_approval_age_basis".to_string(), json!("unknown"));
        return;
    };
    let Some(pool) = pool else {
        evidence.insert(
            "lp_approval_age_basis".to_string(),
            json!("missing_pool_context"),
        );
        return;
    };
    if let Some(can_buy_block) = pool.can_buy_block {
        evidence.insert("trading_enabled_block".to_string(), json!(can_buy_block));
        evidence.insert(
            "trading_enabled_age_blocks_at_signal".to_string(),
            json!(observed_block as i64 - can_buy_block as i64),
        );
        evidence.insert(
            "lp_approval_age_basis".to_string(),
            json!("trading_enabled_block"),
        );
    }
    if let Some(creation_block) = pool.creation_block {
        evidence.insert("pool_creation_block".to_string(), json!(creation_block));
        evidence.insert(
            "pool_age_blocks_at_signal".to_string(),
            json!(observed_block as i64 - creation_block as i64),
        );
        if !evidence.contains_key("lp_approval_age_basis") {
            evidence.insert(
                "lp_approval_age_basis".to_string(),
                json!("pool_creation_block"),
            );
        }
    }
    if !evidence.contains_key("lp_approval_age_basis") {
        evidence.insert("lp_approval_age_basis".to_string(), json!("unknown"));
    }
}
