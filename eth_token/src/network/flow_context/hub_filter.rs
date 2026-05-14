//! Noisy-node suppression for the flow-context backbone.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    config::FlowContextConfig,
    model::{FlowContextEdge, SuppressedHub},
};

/// Result of suppressing high-degree non-seed hubs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HubFilterResult {
    pub edges: Vec<FlowContextEdge>,
    pub suppressed_hubs: Vec<SuppressedHub>,
}

pub fn suppress_noisy_hubs(
    edges: Vec<FlowContextEdge>,
    seed_addresses: &BTreeSet<String>,
    config: &FlowContextConfig,
) -> HubFilterResult {
    let mut degree = BTreeMap::<String, usize>::new();
    for edge in &edges {
        *degree.entry(edge.source.clone()).or_default() += 1;
        *degree.entry(edge.target.clone()).or_default() += 1;
    }

    let suppressed = degree
        .iter()
        .filter_map(|(address, degree)| {
            if seed_addresses.contains(address) || *degree < config.hub_degree_threshold {
                return None;
            }
            Some(address.clone())
        })
        .collect::<BTreeSet<_>>();

    if suppressed.is_empty() {
        return HubFilterResult {
            edges,
            suppressed_hubs: Vec::new(),
        };
    }

    let mut affected_by_hub = BTreeMap::<String, usize>::new();
    let mut kept_edges = Vec::new();
    for edge in edges {
        let suppressed_source = suppressed.contains(&edge.source);
        let suppressed_target = suppressed.contains(&edge.target);
        if suppressed_source || suppressed_target {
            if suppressed_source {
                *affected_by_hub.entry(edge.source.clone()).or_default() += 1;
            }
            if suppressed_target {
                *affected_by_hub.entry(edge.target.clone()).or_default() += 1;
            }
        } else {
            kept_edges.push(edge);
        }
    }

    let suppressed_hubs = suppressed
        .into_iter()
        .map(|address| SuppressedHub {
            degree: degree.get(&address).copied().unwrap_or_default(),
            affected_edge_count: affected_by_hub.get(&address).copied().unwrap_or_default(),
            address,
            reason: "non-seed high-degree fund-flow hub".to_string(),
        })
        .collect();

    HubFilterResult {
        edges: kept_edges,
        suppressed_hubs,
    }
}
