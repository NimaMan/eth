use eth_alpha_core::order::{OrderIntent, OrderSide};

use super::shadow_outcome::ShadowGasSelection;

#[derive(Default)]
pub(super) struct TailEntryOrderingEvidence {
    pub(super) tail_after_tx_hash: Option<String>,
    pub(super) dependency_priority_fee_wei: Option<String>,
    pub(super) dependency_gas_price_wei: Option<String>,
}

pub(super) fn tail_entry_ordering_evidence(
    intent: &OrderIntent,
    selection: &ShadowGasSelection,
) -> Option<TailEntryOrderingEvidence> {
    if intent.side != OrderSide::Buy || selection.action() != "tail_entry_buy" {
        return None;
    }
    let details = &intent.decision_reason.as_ref()?.details;
    let dependency_fee_metadata = details
        .get("risk_event_evidence")?
        .get("mempool_entry_evidence")?
        .get("dependency_fee_metadata")?;
    Some(TailEntryOrderingEvidence {
        tail_after_tx_hash: json_string_field(dependency_fee_metadata, "tail_after_tx_hash"),
        dependency_priority_fee_wei: json_string_field(
            dependency_fee_metadata,
            "dependency_priority_fee_wei",
        ),
        dependency_gas_price_wei: json_string_field(
            dependency_fee_metadata,
            "dependency_gas_price_wei",
        ),
    })
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    let value = value.get(key)?;
    if value.is_null() {
        return None;
    }
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| Some(value.to_string()))
}
