//! Compact API/debug snapshot for the flow-context layer.

use serde::{Deserialize, Serialize};

use super::model::{FlowContextCluster, FlowContextEdge, FlowContextLayer, SuppressedHub};

/// Summary counts for second-order fund-flow context.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FlowContextSummary {
    pub seed_count: usize,
    pub inspected_block_count: usize,
    pub node_count: usize,
    pub edge_count: usize,
    pub path_count: usize,
    pub cluster_count: usize,
    pub suppressed_hub_count: usize,
}

/// Stable snapshot shape for API/debug callers.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FlowContextSnapshot {
    pub summary: FlowContextSummary,
    pub edges: Vec<FlowContextEdge>,
    pub clusters: Vec<FlowContextCluster>,
    pub suppressed_hubs: Vec<SuppressedHub>,
}

impl FlowContextSnapshot {
    pub fn from_layer(layer: &FlowContextLayer) -> Self {
        Self {
            summary: FlowContextSummary {
                seed_count: layer.seed_count,
                inspected_block_count: layer.inspected_block_count,
                node_count: layer.nodes.len(),
                edge_count: layer.edges.len(),
                path_count: layer.paths.len(),
                cluster_count: layer.clusters.len(),
                suppressed_hub_count: layer.suppressed_hubs.len(),
            },
            edges: layer.edges.clone(),
            clusters: layer.clusters.clone(),
            suppressed_hubs: layer.suppressed_hubs.clone(),
        }
    }
}
