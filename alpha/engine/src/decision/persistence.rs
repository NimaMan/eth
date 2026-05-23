use eth_alpha_core::{
    error::Result,
    market::MarketEvent,
    risk::{RiskEvent, RiskPolicy},
    store::TradingStore,
    strategy::StrategyDecision,
};

use crate::{AlphaEngine, EngineExecutionAdapter};

use super::{risk_kind_key, strategy_decision_record};
use serde_json::{json, Map, Value};

impl<E, R, S> AlphaEngine<E, R, S>
where
    E: EngineExecutionAdapter,
    R: RiskPolicy,
    S: TradingStore,
{
    pub(crate) async fn record_strategy_decision(
        &self,
        strategy_name: &str,
        event_source: &str,
        event: &MarketEvent,
        decision: &StrategyDecision,
    ) -> Result<()> {
        let (block_number, token_address, pool_address, event_key) = match event {
            MarketEvent::TokenUpdated {
                block_number,
                token,
            } => (
                Some(*block_number),
                Some(token.address.to_string()),
                None,
                format!("token:{}:{block_number}", token.address),
            ),
            MarketEvent::PoolUpdated { block_number, pool } => (
                Some(*block_number),
                Some(pool.token_address.to_string()),
                Some(pool.address.to_string()),
                format!("pool:{}:{block_number}", pool.address),
            ),
            MarketEvent::BlockCompleted { block_number, .. } => (
                Some(*block_number),
                self.market
                    .as_ref()
                    .map(|market| market.token_address.to_string()),
                self.market
                    .as_ref()
                    .and_then(|market| market.pool_address.as_ref().map(ToString::to_string)),
                format!("block_completed:{block_number}"),
            ),
        };
        self.store
            .record_strategy_decision(&strategy_decision_record(
                strategy_name,
                event_source,
                event_key,
                block_number,
                token_address,
                pool_address,
                decision,
            ))
            .await
    }

    pub(crate) async fn record_position_monitor_decision(
        &self,
        strategy_name: &str,
        block_number: u64,
        index: usize,
        decision: &StrategyDecision,
    ) -> Result<()> {
        let intent = decision.order_intent();
        self.store
            .record_strategy_decision(&strategy_decision_record(
                strategy_name,
                "position_monitor",
                format!("position_monitor:{block_number}:{index}"),
                Some(block_number),
                intent.map(|intent| intent.token_address.to_string()),
                intent.map(|intent| intent.pool_address.to_string()),
                decision,
            ))
            .await
    }

    pub(crate) async fn record_risk_strategy_decision(
        &self,
        strategy_name: &str,
        event: &RiskEvent,
        decision: &StrategyDecision,
    ) -> Result<()> {
        let event_source = event.source.as_deref().unwrap_or("risk");
        let mut record = strategy_decision_record(
            strategy_name,
            event_source,
            format!(
                "risk:{}:{}:{}",
                risk_kind_key(&event.kind),
                event.token_address,
                event.observed_block.unwrap_or_default()
            ),
            event.observed_block,
            Some(event.token_address.to_string()),
            event.pool_address.as_ref().map(ToString::to_string),
            decision,
        );
        attach_risk_event_evidence(&mut record.reason_details, &mut record.payload, event);
        self.store.record_strategy_decision(&record).await
    }
}

fn attach_risk_event_evidence(
    reason_details: &mut Option<Value>,
    payload: &mut Value,
    event: &RiskEvent,
) {
    let Some(evidence) = event.evidence.as_ref() else {
        return;
    };
    let details = reason_details.get_or_insert_with(|| json!({}));
    if !details.is_object() {
        *details = json!({});
    }
    if let Some(map) = details.as_object_mut() {
        map.insert("risk_event_evidence".to_string(), evidence.clone());
        copy_evidence_field(map, evidence, "lp_approval_age_basis", "age_basis");
        copy_evidence_field(
            map,
            evidence,
            "trading_enabled_age_blocks_at_signal",
            "trading_enabled_age_blocks",
        );
        copy_evidence_field(
            map,
            evidence,
            "pool_age_blocks_at_signal",
            "pool_age_blocks",
        );
        copy_evidence_field(
            map,
            evidence,
            "trading_enabled_block",
            "trading_enabled_block",
        );
        copy_evidence_field(map, evidence, "pool_creation_block", "pool_creation_block");
        copy_evidence_field(map, evidence, "observed_block", "risk_observed_block");
        copy_evidence_field(map, evidence, "signal_id", "signal_id");
        copy_evidence_field(
            map,
            evidence,
            "mempool_first_seen_at",
            "mempool_first_seen_at",
        );
    }
    if !payload.is_object() {
        *payload = json!({});
    }
    if let Some(map) = payload.as_object_mut() {
        map.insert("risk_event_evidence".to_string(), evidence.clone());
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
