use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::pools::base::BasePool;
use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ActivityTrack {
    pub swap_count: u64,
    pub mint_count: u64,
    pub burn_count: u64,
    pub buy_volume_denom: f64,
    pub sell_volume_denom: f64,
    pub denom_volume_in: f64,
    pub token_volume_in: f64,
    pub denom_volume_out: f64,
    pub token_volume_out: f64,
    pub latest_swap_block: Option<u64>,
    pub evidence: Vec<EvidenceRef>,
}

impl ActivityTrack {
    pub fn from_base_pool(base: &BasePool) -> Self {
        Self {
            swap_count: base.state.total_swaps,
            mint_count: base.state.total_mints,
            burn_count: base.state.total_burns,
            buy_volume_denom: base.state.denom_volume_in,
            sell_volume_denom: base.state.denom_volume_out,
            denom_volume_in: base.state.denom_volume_in,
            token_volume_in: base.state.token_volume_in,
            denom_volume_out: base.state.denom_volume_out,
            token_volume_out: base.state.token_volume_out,
            latest_swap_block: latest_event_block(&base.swap_events),
            evidence: vec![EvidenceRef::new(
                EvidenceSourceKind::PoolEvent,
                EvidenceConfidence::High,
            )
            .at_block(latest_event_block(&base.swap_events))],
        }
    }
}

fn latest_event_block(events: &[Value]) -> Option<u64> {
    events.iter().filter_map(event_block).max()
}

fn event_block(event: &Value) -> Option<u64> {
    event
        .get("block_number")
        .and_then(Value::as_u64)
        .or_else(|| event.get("block").and_then(Value::as_u64))
}
