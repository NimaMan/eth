mod persistence;
mod record;
mod risk_evidence;

pub(crate) use record::{risk_kind_key, strategy_decision_action};
// `strategy_decision_record` is re-exported `pub` (hidden) at the crate root so
// the live runner's operator manual-close flow can build decision records that
// match the engine's persistence shape; the rest of `decision` stays internal.
#[doc(hidden)]
pub use record::strategy_decision_record;
pub(crate) use risk_evidence::{
    attach_risk_event_evidence_to_payload, enrich_decision_reason_with_risk_event,
    enrich_reason_details_with_risk_event,
};
