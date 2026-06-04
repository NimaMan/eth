use eth_alpha_core::{
    market::PoolSnapshot,
    risk::{RiskEvent, RiskKind, RiskSeverity},
};
use serde_json::{json, Map, Value};

use eth_alpha_engine::wire::PoolWire;

use super::MINED_POOL_RISK_SOURCE;

#[derive(Clone, Debug)]
pub(super) struct MinedPoolRiskCandidate {
    pub key: String,
    pub event: RiskEvent,
}

pub(super) fn mined_pool_risks_from_update(
    pool_wire: &PoolWire,
    pool: &PoolSnapshot,
) -> Vec<MinedPoolRiskCandidate> {
    let mut risks = Vec::new();
    if let Some(candidate) = mined_lp_approval(pool_wire, pool) {
        risks.push(candidate);
    }
    if let Some(candidate) = mined_liquidity_removal(pool_wire, pool) {
        risks.push(candidate);
    }
    risks.sort_by_key(|candidate| {
        (
            candidate.event.observed_block.unwrap_or(pool.latest_block),
            risk_kind_order(&candidate.event.kind),
        )
    });
    risks
}

fn mined_lp_approval(pool_wire: &PoolWire, pool: &PoolSnapshot) -> Option<MinedPoolRiskCandidate> {
    let block = positive_block(pool_wire.lp_last_approval_block)?;
    let count = pool_wire.lp_approval_count.unwrap_or_default();
    let tx_hash = tx_hash_from_value(pool_wire.lp_last_approval.as_ref());
    let key = mined_risk_key("lp_approval", pool, block, tx_hash.as_deref(), Some(count));
    let approved_pct = pool_wire
        .lp_approved_percentage
        .filter(|pct| pct.is_finite());
    let severity = match approved_pct {
        Some(pct) if pct >= 30.0 => RiskSeverity::Critical,
        _ => RiskSeverity::Warning,
    };
    let message = match approved_pct {
        Some(pct) if pct > 0.0 => {
            format!("mined LP approval at block {block} approved_pct={pct}% count={count}")
        }
        _ => format!("mined LP approval at block {block} count={count} approved_pct=unknown"),
    };
    let mut evidence = base_evidence(pool_wire, pool, block);
    evidence.insert("risk_kind".to_string(), json!("lp_approval"));
    evidence.insert("lp_last_approval_block".to_string(), json!(block));
    evidence.insert("lp_approval_count".to_string(), json!(count));
    evidence.insert(
        "lp_last_approval".to_string(),
        json!(pool_wire.lp_last_approval),
    );
    evidence.insert(
        "lp_approved_percentage_raw".to_string(),
        json!(pool_wire.lp_approved_percentage),
    );
    if let Some(pct) = approved_pct.filter(|pct| *pct > 0.0) {
        evidence.insert("approved_share_pct".to_string(), json!(pct));
    }
    if let Some(tx_hash) = tx_hash.as_ref() {
        evidence.insert("lp_approval_tx_hash".to_string(), json!(tx_hash));
    }

    Some(MinedPoolRiskCandidate {
        key,
        event: RiskEvent {
            kind: RiskKind::LpApproval,
            severity,
            source: Some(MINED_POOL_RISK_SOURCE.to_string()),
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            pending_tx_hash: None,
            observed_block: Some(block),
            message,
            evidence: Some(Value::Object(evidence)),
        },
    })
}

fn mined_liquidity_removal(
    pool_wire: &PoolWire,
    pool: &PoolSnapshot,
) -> Option<MinedPoolRiskCandidate> {
    if !pool_wire.liquidity_removal {
        return None;
    }
    let block = positive_block(pool_wire.liquidity_removal_block)?;
    let tx_hash = pool_wire
        .liquidity_removal_tx_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    // Custody-class drains surface through the same liquidity-removal flag (the
    // token tracker marks the pool scam without a reserve move) but carry a
    // distinct label. Tag the evidence risk_kind so backtest validation and
    // operator review can tell the variants apart. Valuation does not depend on
    // this tag — RiskKind::LiquidityRemoval already drives the drained/zero-value
    // path regardless — but the classification feeds analytics.
    let label = pool_wire
        .liquidity_removal_label
        .as_deref()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let is_buyer_confiscation = label.contains("confiscation") || label.contains("custody");
    let is_holder_balance_drain =
        label.contains("holder-balance") || label.contains("holder_balance");
    let risk_kind_tag = if is_buyer_confiscation {
        "custody_buyer_token_confiscation"
    } else if is_holder_balance_drain {
        "holder_balance_backdoor_drain"
    } else {
        "liquidity_removal"
    };
    let is_custody_drain = is_buyer_confiscation || is_holder_balance_drain;
    let key = mined_risk_key(risk_kind_tag, pool, block, tx_hash, None);
    let mut evidence = base_evidence(pool_wire, pool, block);
    evidence.insert("risk_kind".to_string(), json!(risk_kind_tag));
    evidence.insert("liquidity_removal".to_string(), json!(true));
    evidence.insert("liquidity_removal_block".to_string(), json!(block));
    evidence.insert(
        "liquidity_removal_tx_hash".to_string(),
        json!(pool_wire.liquidity_removal_tx_hash),
    );
    evidence.insert(
        "liquidity_removal_label".to_string(),
        json!(pool_wire.liquidity_removal_label),
    );

    Some(MinedPoolRiskCandidate {
        key,
        event: RiskEvent {
            kind: RiskKind::LiquidityRemoval,
            severity: RiskSeverity::Critical,
            source: Some(MINED_POOL_RISK_SOURCE.to_string()),
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            pending_tx_hash: None,
            observed_block: Some(block),
            message: if is_buyer_confiscation {
                format!("backdoor buyer-token confiscation rug at block {block}")
            } else if is_custody_drain {
                format!("holder-balance backdoor drain at block {block}")
            } else {
                format!("mined liquidity removal at block {block}")
            },
            evidence: Some(Value::Object(evidence)),
        },
    })
}

fn base_evidence(
    pool_wire: &PoolWire,
    pool: &PoolSnapshot,
    observed_block: u64,
) -> Map<String, Value> {
    let pool_creation_block = pool.creation_block.or(pool_wire.creation_block);
    let trading_enabled_block = pool_wire.can_buy_block.filter(|block| *block > 0);
    let mut evidence = Map::new();
    evidence.insert("source".to_string(), json!(MINED_POOL_RISK_SOURCE));
    evidence.insert("observed_block".to_string(), json!(observed_block));
    evidence.insert(
        "source_pool_latest_block".to_string(),
        json!(pool.latest_block),
    );
    evidence.insert(
        "pool_creation_block".to_string(),
        json!(pool_creation_block),
    );
    evidence.insert(
        "trading_enabled_block".to_string(),
        json!(trading_enabled_block),
    );
    evidence.insert(
        "pool_age_blocks_at_signal".to_string(),
        json!(block_delta(observed_block, pool_creation_block)),
    );
    evidence.insert(
        "trading_enabled_age_blocks_at_signal".to_string(),
        json!(block_delta(observed_block, trading_enabled_block)),
    );
    evidence.insert(
        "lp_approval_age_basis".to_string(),
        json!(if trading_enabled_block.is_some() {
            "trading_enabled_block"
        } else {
            "pool_creation_block"
        }),
    );
    evidence.insert(
        "token_address".to_string(),
        json!(pool.token_address.to_string()),
    );
    evidence.insert("pool_address".to_string(), json!(pool.address.to_string()));
    evidence.insert(
        "denom_reserve".to_string(),
        json!(pool.denom_reserve.to_string()),
    );
    evidence.insert(
        "token_reserve".to_string(),
        json!(pool.token_reserve.to_string()),
    );
    evidence
}

fn mined_risk_key(
    kind: &str,
    pool: &PoolSnapshot,
    block: u64,
    tx_hash: Option<&str>,
    count: Option<u64>,
) -> String {
    if let Some(tx_hash) = tx_hash {
        format!(
            "{kind}:{}:{block}:{}",
            pool.address,
            tx_hash.to_ascii_lowercase()
        )
    } else if let Some(count) = count {
        format!("{kind}:{}:{block}:count:{count}", pool.address)
    } else {
        format!("{kind}:{}:{block}", pool.address)
    }
}

fn tx_hash_from_value(value: Option<&Value>) -> Option<String> {
    let value = value?;
    ["tx_hash", "transaction_hash", "hash"]
        .into_iter()
        .filter_map(|key| value.get(key).and_then(Value::as_str))
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

fn positive_block(value: Option<u64>) -> Option<u64> {
    value.filter(|block| *block > 0)
}

fn block_delta(observed_block: u64, reference_block: Option<u64>) -> Option<i64> {
    reference_block.map(|reference| observed_block as i64 - reference as i64)
}

fn risk_kind_order(kind: &RiskKind) -> u8 {
    match kind {
        RiskKind::LpApproval => 0,
        RiskKind::LiquidityRemoval => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool_wire(extra: Value) -> PoolWire {
        let mut base = json!({
            "token_address": "0x1111111111111111111111111111111111111111",
            "pool_address": "0x2222222222222222222222222222222222222222",
            "protocol": "UNISWAP-V2",
            "denom_address": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
            "denom_symbol": "WETH",
            "denom_reserve": 1.0,
            "token_reserve": 100.0,
            "price": 0.01,
            "initial_price": 0.01,
            "price_ratio_to_initial": 1.0,
            "creation_block": 10,
            "can_buy_block": 11,
            "latest_block_number": 15,
            "can_buy": true,
            "can_sell": true,
            "is_scam": false
        });
        let base_object = base.as_object_mut().expect("base pool json");
        for (key, value) in extra.as_object().expect("extra pool json") {
            base_object.insert(key.clone(), value.clone());
        }
        serde_json::from_value(base).expect("pool wire")
    }

    #[test]
    fn emits_mined_lp_approval_with_age_evidence() {
        let wire = pool_wire(json!({
            "lp_last_approval_block": 14,
            "lp_approval_count": 1,
            "lp_approved_percentage": 100.0,
            "lp_last_approval": {
                "tx_hash": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }
        }));
        let pool = wire.to_pool_snapshot().expect("pool snapshot");

        let risks = mined_pool_risks_from_update(&wire, &pool);

        assert_eq!(risks.len(), 1);
        assert!(risks[0].key.contains("lp_approval:"));
        let event = &risks[0].event;
        assert_eq!(event.kind, RiskKind::LpApproval);
        assert_eq!(event.severity, RiskSeverity::Critical);
        assert_eq!(event.source.as_deref(), Some(MINED_POOL_RISK_SOURCE));
        assert_eq!(event.observed_block, Some(14));
        assert!(event.pending_tx_hash.is_none());
        let evidence = event.evidence.as_ref().expect("evidence");
        assert_eq!(
            evidence["lp_approval_tx_hash"],
            json!("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert_eq!(evidence["approved_share_pct"], json!(100.0));
        assert_eq!(evidence["trading_enabled_age_blocks_at_signal"], json!(3));
        assert_eq!(evidence["pool_age_blocks_at_signal"], json!(4));
    }

    #[test]
    fn emits_mined_liquidity_removal_with_tx_hash() {
        let wire = pool_wire(json!({
            "liquidity_removal": true,
            "liquidity_removal_block": 15,
            "liquidity_removal_tx_hash": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }));
        let pool = wire.to_pool_snapshot().expect("pool snapshot");

        let risks = mined_pool_risks_from_update(&wire, &pool);

        assert_eq!(risks.len(), 1);
        assert!(risks[0].key.contains("liquidity_removal:"));
        let event = &risks[0].event;
        assert_eq!(event.kind, RiskKind::LiquidityRemoval);
        assert_eq!(event.severity, RiskSeverity::Critical);
        assert!(event.pending_tx_hash.is_none());
        assert_eq!(
            event.evidence.as_ref().unwrap()["liquidity_removal"],
            json!(true)
        );
        assert_eq!(
            event.evidence.as_ref().unwrap()["liquidity_removal_tx_hash"],
            json!("0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        );
    }

    #[test]
    fn orders_lp_approval_before_liquidity_removal() {
        let wire = pool_wire(json!({
            "lp_last_approval_block": 14,
            "lp_approval_count": 1,
            "liquidity_removal": true,
            "liquidity_removal_block": 15
        }));
        let pool = wire.to_pool_snapshot().expect("pool snapshot");

        let risks = mined_pool_risks_from_update(&wire, &pool);

        assert_eq!(risks.len(), 2);
        assert_eq!(risks[0].event.kind, RiskKind::LpApproval);
        assert_eq!(risks[1].event.kind, RiskKind::LiquidityRemoval);
    }
}
