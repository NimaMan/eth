use std::collections::{BTreeMap, BTreeSet};

use crate::read_models::token_analytics::TokenNetworkGraphView;
use eth_token::network::{
    activity::AddressActivity,
    flow_context::{
        build_layer_from_observations, snapshot::FlowContextSnapshot, FlowContextBuildArtifacts,
        FlowContextConfig,
    },
    graph::RawTokenNetworkGraph,
    model::{NetworkEdge, NetworkNode, NetworkNodeId},
};
use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
pub struct TokenNetworkAnalysisTimeline {
    pub active_blocks: Vec<u64>,
    pub frame_count: usize,
    pub frames: Vec<TokenNetworkAnalysisTimelineFrame>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenNetworkAnalysisTimelineFrame {
    pub index: usize,
    pub block_number: u64,
    pub block_timestamp: Option<u64>,
    pub token_graph: TimelineTokenGraphFrame,
    pub flow_context: FlowContextSnapshot,
    pub deltas: TokenNetworkAnalysisTimelineDelta,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TokenNetworkAnalysisTimelineDelta {
    pub token_node_delta: isize,
    pub token_edge_delta: isize,
    pub token_address_delta: isize,
    pub flow_node_delta: isize,
    pub flow_edge_delta: isize,
    pub flow_cluster_delta: isize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TimelineTokenGraphFrame {
    pub node_count: usize,
    pub edge_count: usize,
    pub address_count: usize,
    pub graph: TokenNetworkGraphView,
}

pub fn build_timeline(
    graph: &RawTokenNetworkGraph,
    flow_config: &FlowContextConfig,
    flow_artifacts: &FlowContextBuildArtifacts,
    active_blocks: &[u64],
    block_timestamps: &BTreeMap<u64, u64>,
) -> TokenNetworkAnalysisTimeline {
    let active_blocks = sorted_unique_blocks(active_blocks);
    let mut frames = Vec::with_capacity(active_blocks.len());
    let mut previous_counts = TimelineCounts::default();

    for (index, block_number) in active_blocks.iter().copied().enumerate() {
        let token_graph = token_graph_frame_until(graph, block_number);
        let flow_context = flow_snapshot_until(flow_config, flow_artifacts, block_number);
        let counts = TimelineCounts::from_frame(&token_graph, &flow_context);
        let deltas = counts.delta(previous_counts);
        previous_counts = counts;

        frames.push(TokenNetworkAnalysisTimelineFrame {
            index,
            block_number,
            block_timestamp: block_timestamps.get(&block_number).copied(),
            token_graph,
            flow_context,
            deltas,
        });
    }

    TokenNetworkAnalysisTimeline {
        frame_count: frames.len(),
        active_blocks,
        frames,
    }
}

fn sorted_unique_blocks(blocks: &[u64]) -> Vec<u64> {
    blocks
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn token_graph_frame_until(
    graph: &RawTokenNetworkGraph,
    block_number: u64,
) -> TimelineTokenGraphFrame {
    let mut visible_nodes = graph
        .nodes
        .iter()
        .filter_map(|(id, node)| {
            if node_visible_until(node, block_number) {
                Some((id.clone(), node.clone()))
            } else {
                None
            }
        })
        .collect::<BTreeMap<_, _>>();

    let visible_edges = graph
        .edges
        .values()
        .filter(|edge| {
            edge_first_seen_block(edge)
                .map(|block| block <= block_number)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();

    for edge in &visible_edges {
        for node_id in [edge.source(), edge.target()] {
            let key = node_id.stable_key();
            if let Some(node) = graph.nodes.get(&key) {
                visible_nodes.entry(key).or_insert_with(|| node.clone());
            }
        }
    }

    let address_activity = graph
        .address_activity
        .iter()
        .filter_map(|(address, activity)| {
            activity_until(activity, block_number).map(|activity| (address.clone(), activity))
        })
        .collect::<BTreeMap<_, _>>();

    let summary_by_node = address_activity
        .values()
        .map(|activity| {
            let summary = activity.summary(None, None);
            (summary.node_id.stable_key(), summary)
        })
        .collect::<BTreeMap<_, _>>();

    let graph_view =
        TokenNetworkGraphView::from_parts(&visible_nodes, &visible_edges, &summary_by_node);

    TimelineTokenGraphFrame {
        node_count: visible_nodes.len(),
        edge_count: visible_edges.len(),
        address_count: address_activity.len(),
        graph: graph_view,
    }
}

fn flow_snapshot_until(
    config: &FlowContextConfig,
    artifacts: &FlowContextBuildArtifacts,
    block_number: u64,
) -> FlowContextSnapshot {
    let seeds = artifacts
        .seeds
        .iter()
        .filter(|seed| {
            seed.first_seen_block
                .or(seed.last_seen_block)
                .map(|block| block <= block_number)
                .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();
    let seed_addresses = seeds
        .iter()
        .map(|seed| seed.address.clone())
        .collect::<BTreeSet<_>>();
    let observations = artifacts
        .observations
        .iter()
        .filter(|observation| observation.observation.block_number <= block_number)
        .cloned()
        .collect::<Vec<_>>();
    let inspected_block_count = artifacts
        .selected_block_numbers
        .iter()
        .filter(|selected| **selected <= block_number)
        .count();
    let layer = build_layer_from_observations(
        config,
        &seeds,
        &seed_addresses,
        inspected_block_count,
        &observations,
    );
    FlowContextSnapshot::from_layer(&layer)
}

#[derive(Clone, Copy, Debug, Default)]
struct TimelineCounts {
    token_nodes: usize,
    token_edges: usize,
    token_addresses: usize,
    flow_nodes: usize,
    flow_edges: usize,
    flow_clusters: usize,
}

impl TimelineCounts {
    fn from_frame(
        token_graph: &TimelineTokenGraphFrame,
        flow_context: &FlowContextSnapshot,
    ) -> Self {
        Self {
            token_nodes: token_graph.node_count,
            token_edges: token_graph.edge_count,
            token_addresses: token_graph.address_count,
            flow_nodes: flow_context.summary.node_count,
            flow_edges: flow_context.summary.edge_count,
            flow_clusters: flow_context.summary.cluster_count,
        }
    }

    fn delta(self, previous: Self) -> TokenNetworkAnalysisTimelineDelta {
        TokenNetworkAnalysisTimelineDelta {
            token_node_delta: usize_delta(self.token_nodes, previous.token_nodes),
            token_edge_delta: usize_delta(self.token_edges, previous.token_edges),
            token_address_delta: usize_delta(self.token_addresses, previous.token_addresses),
            flow_node_delta: usize_delta(self.flow_nodes, previous.flow_nodes),
            flow_edge_delta: usize_delta(self.flow_edges, previous.flow_edges),
            flow_cluster_delta: usize_delta(self.flow_clusters, previous.flow_clusters),
        }
    }
}

fn node_visible_until(node: &NetworkNode, block_number: u64) -> bool {
    matches!(node.id, NetworkNodeId::Token(_))
        || node
            .observed
            .first_seen
            .as_ref()
            .map(|observation| observation.block_number <= block_number)
            .unwrap_or(false)
}

fn edge_first_seen_block(edge: &NetworkEdge) -> Option<u64> {
    edge.evidence
        .observed
        .first_seen
        .as_ref()
        .map(|observation| observation.block_number)
        .or_else(|| {
            edge.evidence
                .examples
                .iter()
                .filter_map(|evidence| evidence.observation.as_ref())
                .map(|observation| observation.block_number)
                .min()
        })
}

fn activity_until(activity: &AddressActivity, block_number: u64) -> Option<AddressActivity> {
    let mut filtered =
        AddressActivity::with_history_limit(&activity.address, activity.history_limit);
    if activity.is_fee_source {
        filtered.mark_fee_source();
    }
    if let Some(fee_source) = &activity.fee_source {
        filtered.set_fee_source(fee_source);
    }
    for movement in &activity.movements {
        if movement.observation.block_number <= block_number {
            filtered.record_movement(movement.clone());
        }
    }
    for cost in &activity.costs {
        if cost.observation.block_number <= block_number {
            filtered.record_cost(cost.clone());
        }
    }
    (filtered.observed.count > 0).then_some(filtered)
}

fn usize_delta(current: usize, previous: usize) -> isize {
    current as isize - previous as isize
}

#[cfg(test)]
mod tests {
    use eth_token::network::{
        flow_context::{
            FlowContextEdgeKind, FlowContextObservation, FlowContextSeed, FlowContextSeedKind,
        },
        ingest::NetworkIngestBatch,
        model::{
            NetworkEdge, NetworkEdgeKind, NetworkEvidence, NetworkEvidenceSource, NetworkNodeId,
            NetworkObservation, TokenNetworkId,
        },
    };

    use super::*;

    fn observation(block: u64) -> NetworkObservation {
        NetworkObservation::new(block)
    }

    fn address(index: u8) -> String {
        format!("0x{index:040x}")
    }

    fn seed(index: u8, first_seen_block: u64) -> FlowContextSeed {
        FlowContextSeed {
            address: address(index),
            kind: FlowContextSeedKind::ActiveTrader,
            score: 1.0,
            labels: Vec::new(),
            first_seen_block: Some(first_seen_block),
            last_seen_block: Some(first_seen_block),
        }
    }

    fn graph_with_edge(edge_block: u64) -> RawTokenNetworkGraph {
        let mut graph = RawTokenNetworkGraph::new(TokenNetworkId::new(None, address(99)));
        let mut edge = NetworkEdge::new(
            NetworkNodeId::address(address(1)),
            NetworkNodeId::address(address(2)),
            NetworkEdgeKind::TokenTransfer,
        );
        edge.record_evidence(
            NetworkEvidence::new(NetworkEvidenceSource::TokenTransfer)
                .with_observation(observation(edge_block)),
        );
        let mut batch = NetworkIngestBatch::default();
        batch.push_edge(edge);
        graph.apply_batch(batch);
        graph
    }

    fn artifacts(observations: Vec<FlowContextObservation>) -> FlowContextBuildArtifacts {
        let seeds = vec![seed(1, 1), seed(2, 1)];
        let seed_addresses = seeds
            .iter()
            .map(|seed| seed.address.clone())
            .collect::<BTreeSet<_>>();
        FlowContextBuildArtifacts {
            layer: Default::default(),
            seeds,
            seed_addresses,
            selected_block_numbers: vec![10, 20, 30],
            observations,
        }
    }

    #[test]
    fn timeline_frames_are_sorted_by_active_block() {
        let graph = graph_with_edge(20);
        let timeline = build_timeline(
            &graph,
            &FlowContextConfig::default(),
            &artifacts(Vec::new()),
            &[30, 10, 20, 20],
            &BTreeMap::new(),
        );

        assert_eq!(timeline.active_blocks, vec![10, 20, 30]);
        assert_eq!(timeline.frames[0].block_number, 10);
        assert_eq!(timeline.frames[2].block_number, 30);
    }

    #[test]
    fn token_edges_are_revealed_cumulatively_by_evidence_block() {
        let graph = graph_with_edge(20);
        let timeline = build_timeline(
            &graph,
            &FlowContextConfig::default(),
            &artifacts(Vec::new()),
            &[10, 20],
            &BTreeMap::new(),
        );

        assert_eq!(timeline.frames[0].token_graph.edge_count, 0);
        assert_eq!(timeline.frames[1].token_graph.edge_count, 1);
        assert_eq!(timeline.frames[1].deltas.token_edge_delta, 1);
    }

    #[test]
    fn empty_active_blocks_make_empty_timeline() {
        let graph = graph_with_edge(20);
        let timeline = build_timeline(
            &graph,
            &FlowContextConfig::default(),
            &artifacts(Vec::new()),
            &[],
            &BTreeMap::new(),
        );

        assert_eq!(timeline.frame_count, 0);
        assert!(timeline.frames.is_empty());
    }

    #[test]
    fn flow_timeline_uses_uncapped_raw_observations() {
        let graph = graph_with_edge(1);
        let observations = vec![
            FlowContextObservation::new(address(1), address(3), observation(20))
                .with_scaled_amount(Some("ETH"), Some("ETH"), 0.1),
            FlowContextObservation::new(address(1), address(3), observation(20))
                .with_scaled_amount(Some("ETH"), Some("ETH"), 0.2),
            FlowContextObservation::new(address(1), address(3), observation(20))
                .with_scaled_amount(Some("ETH"), Some("ETH"), 0.3),
        ];
        let mut config = FlowContextConfig {
            max_evidence_per_edge: 1,
            ..FlowContextConfig::default()
        };
        config.max_context_edges = 10;

        let timeline = build_timeline(
            &graph,
            &config,
            &artifacts(observations),
            &[20],
            &BTreeMap::new(),
        );

        let edge = timeline.frames[0]
            .flow_context
            .edges
            .iter()
            .find(|edge| edge.kind == FlowContextEdgeKind::DirectDenomFlow)
            .expect("direct flow edge exists");
        assert_eq!(edge.weight, 3);
        assert_eq!(edge.evidence.len(), 1);
    }
}
