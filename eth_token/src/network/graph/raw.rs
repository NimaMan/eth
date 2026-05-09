//! Canonical raw token-network graph state.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::network::{
    activity::{AddressActivity, AddressCostRecord, AddressMovement},
    config::TokenNetworkGraphConfig,
    graph::index::NetworkGraphIndex,
    ingest::{AddressCostIngest, AddressMovementIngest, NetworkIngestBatch, NodeLabelIngest},
    model::{
        NetworkEdge, NetworkEdgeAmountSummary, NetworkLabel, NetworkLabelKind, NetworkLabelSource,
        NetworkNode, NetworkNodeId, NetworkNodeKind, TokenNetworkId,
    },
};

/// Persistent raw graph state for one tracked token.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawTokenNetworkGraph {
    pub network_id: TokenNetworkId,
    pub config: TokenNetworkGraphConfig,
    pub nodes: BTreeMap<String, NetworkNode>,
    pub edges: BTreeMap<String, NetworkEdge>,
    pub address_activity: BTreeMap<String, AddressActivity>,
    pub index: NetworkGraphIndex,
    pub applied_batches: u64,
}

impl RawTokenNetworkGraph {
    pub fn new(network_id: TokenNetworkId) -> Self {
        Self::with_config(network_id, TokenNetworkGraphConfig::default())
    }

    pub fn with_config(network_id: TokenNetworkId, config: TokenNetworkGraphConfig) -> Self {
        let mut graph = Self {
            network_id: network_id.clone(),
            config,
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            address_activity: BTreeMap::new(),
            index: NetworkGraphIndex::default(),
            applied_batches: 0,
        };
        graph.ensure_node(NetworkNodeId::token(network_id.token_address));
        graph
    }

    pub fn apply_batch(&mut self, batch: NetworkIngestBatch) {
        for movement in batch.address_movements {
            self.apply_address_movement(movement);
        }
        for cost in batch.address_costs {
            self.apply_address_cost(cost);
        }
        for label in batch.node_labels {
            self.apply_node_label(label);
        }
        for edge in batch.edges {
            self.apply_edge(edge);
        }
        self.applied_batches += 1;
    }

    pub fn ensure_node(&mut self, node_id: NetworkNodeId) -> &mut NetworkNode {
        let key = node_id.stable_key();
        self.index.index_node(&node_id);
        self.nodes
            .entry(key)
            .or_insert_with(|| node_from_id(node_id))
    }

    pub fn node(&self, node_id: &NetworkNodeId) -> Option<&NetworkNode> {
        self.nodes.get(&node_id.stable_key())
    }

    pub fn edge(&self, edge: &NetworkEdge) -> Option<&NetworkEdge> {
        self.edges.get(&edge.id.stable_key())
    }

    pub fn address_activity(&self, address: impl AsRef<str>) -> Option<&AddressActivity> {
        self.address_activity
            .get(&normalize_address(address.as_ref()))
    }

    fn apply_address_movement(&mut self, ingest: AddressMovementIngest) {
        let address = normalize_address(&ingest.address);
        self.ensure_node(NetworkNodeId::address(&address));
        self.record_node_observation(&NetworkNodeId::address(&address), &ingest.movement);
        self.activity_mut(&address).record_movement(ingest.movement);
    }

    fn apply_address_cost(&mut self, ingest: AddressCostIngest) {
        let address = normalize_address(&ingest.address);
        self.ensure_node(NetworkNodeId::address(&address));
        self.record_cost_observation(&NetworkNodeId::address(&address), &ingest.cost);
        self.activity_mut(&address).record_cost(ingest.cost);
    }

    fn apply_node_label(&mut self, ingest: NodeLabelIngest) {
        let node_id = ingest.node_id;
        let node = self.ensure_node(node_id.clone());
        merge_label(node, ingest.label);
        if let Some(evidence) = ingest.evidence {
            if let Some(observation) = evidence.observation {
                node.observed.record(observation);
            }
        }
    }

    fn apply_edge(&mut self, mut incoming: NetworkEdge) {
        self.ensure_node(incoming.id.source.clone());
        self.ensure_node(incoming.id.target.clone());
        incoming.evidence.max_examples = self.config.edge_evidence_limit;
        if incoming.evidence.examples.len() > self.config.edge_evidence_limit {
            incoming
                .evidence
                .examples
                .truncate(self.config.edge_evidence_limit);
        }

        let key = incoming.id.stable_key();
        if let Some(existing) = self.edges.get_mut(&key) {
            merge_edge(existing, incoming);
        } else {
            self.edges.insert(key, incoming);
        }
    }

    fn activity_mut(&mut self, address: &str) -> &mut AddressActivity {
        self.address_activity
            .entry(address.to_string())
            .or_insert_with(|| {
                AddressActivity::with_history_limit(
                    address,
                    self.config.address_activity_history_limit,
                )
            })
    }

    fn record_node_observation(&mut self, node_id: &NetworkNodeId, movement: &AddressMovement) {
        if let Some(node) = self.nodes.get_mut(&node_id.stable_key()) {
            node.observed.record(movement.observation.clone());
        }
    }

    fn record_cost_observation(&mut self, node_id: &NetworkNodeId, cost: &AddressCostRecord) {
        if let Some(node) = self.nodes.get_mut(&node_id.stable_key()) {
            node.observed.record(cost.observation.clone());
        }
    }
}

fn node_from_id(node_id: NetworkNodeId) -> NetworkNode {
    let kind = match &node_id {
        NetworkNodeId::Token(_) => NetworkNodeKind::Token,
        NetworkNodeId::Address(_) => NetworkNodeKind::Address,
        NetworkNodeId::Pool(_) => NetworkNodeKind::Pool,
        NetworkNodeId::TimeWindow(_) => NetworkNodeKind::TimeWindow,
        NetworkNodeId::Synthetic(_) => NetworkNodeKind::Synthetic,
    };
    let mut node = NetworkNode::new(node_id, kind);
    match node.kind {
        NetworkNodeKind::Token => {
            node.add_observed_label(
                NetworkLabelKind::TokenContract,
                NetworkLabelSource::TokenState,
            );
        }
        NetworkNodeKind::Pool => {
            node.add_observed_label(NetworkLabelKind::Pool, NetworkLabelSource::PoolState);
        }
        NetworkNodeKind::TimeWindow => {
            node.add_observed_label(NetworkLabelKind::TimeWindow, NetworkLabelSource::Heuristic);
        }
        _ => {}
    }
    node
}

fn merge_label(node: &mut NetworkNode, label: NetworkLabel) {
    if let Some(existing) = node.labels.iter_mut().find(|existing| {
        existing.kind == label.kind
            && existing.source == label.source
            && existing.value == label.value
    }) {
        existing.observed.merge(&label.observed);
        if existing.confidence.score.is_none() {
            existing.confidence.score = label.confidence.score;
        }
        if existing.confidence.reason.is_none() {
            existing.confidence.reason = label.confidence.reason;
        }
    } else {
        node.labels.push(label);
    }
}

fn merge_edge(existing: &mut NetworkEdge, incoming: NetworkEdge) {
    existing.evidence.merge(incoming.evidence);
    merge_edge_amount(&mut existing.amount, incoming.amount);
    if existing.confidence.is_none() {
        existing.confidence = incoming.confidence;
    }
    for (key, value) in incoming.attributes {
        existing.attributes.insert(key, value);
    }
}

fn merge_edge_amount(
    existing: &mut Option<NetworkEdgeAmountSummary>,
    incoming: Option<NetworkEdgeAmountSummary>,
) {
    let Some(incoming) = incoming else {
        return;
    };
    let Some(existing_amount) = existing else {
        *existing = Some(incoming);
        return;
    };
    if existing_amount.asset == incoming.asset && existing_amount.symbol == incoming.symbol {
        if let (Some(left), Some(right)) = (existing_amount.scaled_total, incoming.scaled_total) {
            existing_amount.scaled_total = Some(left + right);
        }
    }
}

fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{
        activity::AddressMovement,
        ingest::{AddressMovementIngest, NetworkIngestBatch},
        model::{NetworkEdgeKind, NetworkEvidence, NetworkEvidenceSource, NetworkObservation},
    };

    fn graph() -> RawTokenNetworkGraph {
        RawTokenNetworkGraph::new(TokenNetworkId::new(
            Some(1),
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ))
    }

    fn observation(block: u64) -> NetworkObservation {
        NetworkObservation::with_transaction(
            block,
            Some(1_700 + block),
            Some("0xhash"),
            Some(1),
            None,
        )
    }

    #[test]
    fn applies_address_movements_into_activity_and_nodes() {
        let mut graph = graph();
        let mut batch = NetworkIngestBatch::default();
        batch.address_movements.push(AddressMovementIngest::new(
            "0x1111111111111111111111111111111111111111",
            AddressMovement::token_in(10.0, observation(100)),
        ));

        graph.apply_batch(batch);

        assert_eq!(graph.applied_batches, 1);
        assert!(graph
            .node(&NetworkNodeId::address(
                "0x1111111111111111111111111111111111111111"
            ))
            .is_some());
        assert_eq!(
            graph
                .address_activity("0x1111111111111111111111111111111111111111")
                .map(AddressActivity::token_balance),
            Some(10.0)
        );
    }

    #[test]
    fn merges_duplicate_edges_and_preserves_evidence_counts() {
        let mut graph = graph();
        let mut first = NetworkEdge::new(
            NetworkNodeId::address("0x1"),
            NetworkNodeId::address("0x2"),
            NetworkEdgeKind::TokenTransfer,
        );
        first.record_evidence(
            NetworkEvidence::new(NetworkEvidenceSource::TokenTransfer)
                .with_observation(observation(100)),
        );
        let mut second = NetworkEdge::new(
            NetworkNodeId::address("0x1"),
            NetworkNodeId::address("0x2"),
            NetworkEdgeKind::TokenTransfer,
        );
        second.record_evidence(
            NetworkEvidence::new(NetworkEvidenceSource::TokenTransfer)
                .with_observation(observation(101)),
        );

        let mut batch = NetworkIngestBatch::default();
        batch.edges.push(first);
        batch.edges.push(second);
        graph.apply_batch(batch);

        let edge = graph.edges.values().next().expect("edge is present");
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(edge.evidence.observed.count, 2);
        assert_eq!(
            edge.evidence
                .observed
                .last_seen
                .as_ref()
                .map(|observation| observation.block_number),
            Some(101)
        );
    }

    #[test]
    fn history_limit_bounds_activity_examples_not_totals() {
        let mut graph = RawTokenNetworkGraph::with_config(
            TokenNetworkId::new(Some(1), "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            TokenNetworkGraphConfig {
                address_activity_history_limit: 1,
                edge_evidence_limit: 8,
            },
        );
        let address = "0x1111111111111111111111111111111111111111";
        let mut batch = NetworkIngestBatch::default();
        batch.address_movements.push(AddressMovementIngest::new(
            address,
            AddressMovement::token_in(1.0, observation(100)),
        ));
        batch.address_movements.push(AddressMovementIngest::new(
            address,
            AddressMovement::token_in(2.0, observation(101)),
        ));

        graph.apply_batch(batch);

        let activity = graph.address_activity(address).expect("activity exists");
        assert_eq!(activity.movements.len(), 1);
        assert_eq!(activity.token_balance(), 3.0);
    }
}
