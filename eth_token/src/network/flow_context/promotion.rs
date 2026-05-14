//! Optional promotion from flow-context findings into token-network inferred edges.

use crate::network::model::{
    NetworkConfidence, NetworkEdge, NetworkEdgeKind, NetworkEvidence, NetworkEvidenceSource,
    NetworkNodeId,
};

use super::{
    config::FlowContextConfig,
    model::{FlowContextEdge, FlowContextEdgeKind},
};

/// Convert high-confidence second-order context edges into the existing
/// token-network edge taxonomy. This returns edges for a caller to review/apply;
/// it does not mutate the raw graph.
pub fn promote_flow_context_edges(
    edges: &[FlowContextEdge],
    config: &FlowContextConfig,
) -> Vec<NetworkEdge> {
    edges
        .iter()
        .filter(|edge| edge.confidence >= config.min_promotion_confidence)
        .filter_map(promote_edge)
        .collect()
}

fn promote_edge(edge: &FlowContextEdge) -> Option<NetworkEdge> {
    let (kind, source) = match edge.kind {
        FlowContextEdgeKind::DirectDenomFlow => (
            NetworkEdgeKind::DenomTransfer,
            NetworkEvidenceSource::DenomTransfer,
        ),
        FlowContextEdgeKind::SharedFunder | FlowContextEdgeKind::MultiHopFundingPath => {
            (NetworkEdgeKind::Funding, NetworkEvidenceSource::Funding)
        }
        FlowContextEdgeKind::TemporalFunding => (
            NetworkEdgeKind::TemporalCoactivity,
            NetworkEvidenceSource::TemporalCoactivity,
        ),
        FlowContextEdgeKind::SharedSink | FlowContextEdgeKind::ProfitConvergence => (
            NetworkEdgeKind::SharedIntermediary,
            NetworkEvidenceSource::SharedIntermediary,
        ),
        FlowContextEdgeKind::SuppressedHubLink => return None,
    };

    let mut network_edge = NetworkEdge::new(
        NetworkNodeId::address(&edge.source),
        NetworkNodeId::address(&edge.target),
        kind,
    );
    network_edge.confidence = Some(NetworkConfidence::inferred(
        crate::network::model::ConfidenceLevel::Medium,
        Some(edge.confidence),
        edge.explanation
            .clone()
            .unwrap_or_else(|| "flow-context promoted edge".to_string()),
    ));

    let mut evidence = NetworkEvidence::new(source).with_description(
        edge.explanation
            .clone()
            .unwrap_or_else(|| format!("promoted flow-context edge {}", edge.kind.stable_key())),
    );
    if let Some(first) = edge.evidence.first() {
        evidence.observation = Some(first.observation.clone());
        evidence.amount = first
            .scaled_amount
            .map(|amount| crate::network::model::NetworkAmount {
                asset: first.asset.clone(),
                symbol: first.symbol.clone(),
                raw: first.raw_amount.clone(),
                scaled: Some(amount),
            });
    }
    evidence.attributes.insert(
        "flow_context_kind".to_string(),
        edge.kind.stable_key().to_string(),
    );
    evidence
        .attributes
        .insert("confidence".to_string(), edge.confidence.to_string());
    evidence
        .attributes
        .insert("weight".to_string(), edge.weight.to_string());
    if let Some(amount) = edge.total_scaled_amount {
        evidence
            .attributes
            .insert("total_scaled_amount".to_string(), amount.to_string());
    }

    network_edge.record_evidence(evidence);
    Some(network_edge)
}
