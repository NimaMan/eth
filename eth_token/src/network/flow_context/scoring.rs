//! Confidence scoring for flow-context edges and clusters.

use super::model::{FlowContextCluster, FlowContextEdge, FlowContextEdgeKind};

pub fn score_edge(edge: &mut FlowContextEdge) {
    let base = match edge.kind {
        FlowContextEdgeKind::DirectDenomFlow => 0.55,
        FlowContextEdgeKind::SharedFunder => 0.72,
        FlowContextEdgeKind::SharedSink => 0.68,
        FlowContextEdgeKind::MultiHopFundingPath => 0.62,
        FlowContextEdgeKind::TemporalFunding => 0.70,
        FlowContextEdgeKind::ProfitConvergence => 0.74,
        FlowContextEdgeKind::SuppressedHubLink => 0.25,
    };
    let repeat_bonus = ((edge.weight.saturating_sub(1) as f64) * 0.04).min(0.16);
    let amount_bonus = edge
        .total_scaled_amount
        .map(|amount| (amount.abs().log10().max(0.0) * 0.02).min(0.08))
        .unwrap_or(0.0);
    edge.confidence = (base + repeat_bonus + amount_bonus).min(0.98);
    edge.explanation = Some(edge_explanation(edge));
}

pub fn score_edges(edges: &mut [FlowContextEdge]) {
    for edge in edges {
        score_edge(edge);
    }
}

pub fn score_cluster(cluster: &mut FlowContextCluster, edges: &[FlowContextEdge]) {
    let relevant = edges
        .iter()
        .filter(|edge| edge.kind == cluster.kind)
        .filter(|edge| {
            cluster.members.iter().any(|member| member == &edge.source)
                || cluster.members.iter().any(|member| member == &edge.target)
        })
        .collect::<Vec<_>>();
    if relevant.is_empty() {
        return;
    }
    let total = relevant.iter().map(|edge| edge.confidence).sum::<f64>();
    cluster.confidence = total / relevant.len() as f64;
    cluster.edge_count = relevant.len();
}

fn edge_explanation(edge: &FlowContextEdge) -> String {
    match edge.kind {
        FlowContextEdgeKind::DirectDenomFlow => {
            "observed non-token value flow involving a token-network actor"
        }
        FlowContextEdgeKind::SharedFunder => {
            "same upstream funder sent non-token value to multiple token-network actors"
        }
        FlowContextEdgeKind::SharedSink => {
            "multiple token-network actors sent non-token value to the same sink"
        }
        FlowContextEdgeKind::MultiHopFundingPath => {
            "short non-token value path connects token-network actors"
        }
        FlowContextEdgeKind::TemporalFunding => {
            "non-token funding happened near token-network activity"
        }
        FlowContextEdgeKind::ProfitConvergence => {
            "profitable token-network actors converge to the same non-token sink"
        }
        FlowContextEdgeKind::SuppressedHubLink => {
            "relationship passes through a suppressed high-noise hub"
        }
    }
    .to_string()
}
