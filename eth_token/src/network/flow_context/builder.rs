//! Orchestration for second-order fund-flow context construction.

use std::collections::{BTreeMap, BTreeSet};

use eyre::Result;

use crate::network::graph::RawTokenNetworkGraph;

use super::{
    block_loader::ProcessedBlockLoader,
    config::FlowContextConfig,
    extractor::{aggregate_flow_context_edges, FlowContextObservation, FlowObservationExtractor},
    hub_filter::suppress_noisy_hubs,
    index_query::AddressParticipationQuery,
    model::{
        FlowContextCluster, FlowContextEdgeKind, FlowContextLayer, FlowContextNode,
        FlowContextNodeRole,
    },
    scoring::{score_cluster, score_edges},
    seeds::{select_flow_context_seeds, FlowContextSeed},
    windows::build_flow_context_windows,
};

/// Full build output for callers that need to inspect or replay the raw
/// observations, such as block-by-block timeline views.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowContextBuildArtifacts {
    pub layer: FlowContextLayer,
    pub seeds: Vec<FlowContextSeed>,
    pub seed_addresses: BTreeSet<String>,
    pub selected_block_numbers: Vec<u64>,
    pub observations: Vec<FlowContextObservation>,
}

/// Builds a `FlowContextLayer` from pluggable index, block, and extraction
/// adapters.
pub struct FlowContextBuilder<Q, L, X> {
    pub config: FlowContextConfig,
    pub index_query: Q,
    pub block_loader: L,
    pub extractor: X,
}

impl<Q, L, X> FlowContextBuilder<Q, L, X>
where
    Q: AddressParticipationQuery,
    L: ProcessedBlockLoader,
    X: FlowObservationExtractor,
{
    pub fn new(config: FlowContextConfig, index_query: Q, block_loader: L, extractor: X) -> Self {
        Self {
            config,
            index_query,
            block_loader,
            extractor,
        }
    }

    pub fn build(&self, graph: &RawTokenNetworkGraph) -> Result<FlowContextLayer> {
        Ok(self.build_with_artifacts(graph)?.layer)
    }

    pub fn build_with_artifacts(
        &self,
        graph: &RawTokenNetworkGraph,
    ) -> Result<FlowContextBuildArtifacts> {
        let seeds = select_flow_context_seeds(graph, &self.config);
        let seed_addresses = seeds
            .iter()
            .map(|seed| seed.address.clone())
            .collect::<BTreeSet<_>>();
        let windows = build_flow_context_windows(graph, &seeds, &self.config);
        let mut selected_blocks = BTreeSet::new();

        for window in windows {
            let mut blocks = self
                .index_query
                .blocks_for_address(&window.address, window.range)?;
            blocks.truncate(self.config.max_blocks_per_address);
            selected_blocks.extend(blocks);
        }

        let block_numbers = selected_blocks.iter().copied().collect::<Vec<_>>();
        let blocks = self.block_loader.load_blocks(&block_numbers)?;
        let observations = self
            .extractor
            .extract_observations(&blocks, &seed_addresses)?;
        let layer = build_layer_from_observations(
            &self.config,
            &seeds,
            &seed_addresses,
            block_numbers.len(),
            &observations,
        );

        Ok(FlowContextBuildArtifacts {
            layer,
            seeds,
            seed_addresses,
            selected_block_numbers: block_numbers,
            observations,
        })
    }
}

pub fn build_layer_from_observations(
    config: &FlowContextConfig,
    seeds: &[FlowContextSeed],
    seed_addresses: &BTreeSet<String>,
    inspected_block_count: usize,
    observations: &[FlowContextObservation],
) -> FlowContextLayer {
    let edges = aggregate_flow_context_edges(observations, seed_addresses, config);
    let filtered = suppress_noisy_hubs(edges, seed_addresses, config);
    let mut edges = filtered.edges;
    score_edges(&mut edges);

    let mut nodes = nodes_from_seeds_and_edges(seeds, &edges);
    nodes.sort_by(|left, right| {
        left.address
            .cmp(&right.address)
            .then_with(|| left.role.cmp(&right.role))
    });

    let mut clusters = build_connector_clusters(&edges);
    for cluster in &mut clusters {
        score_cluster(cluster, &edges);
    }

    FlowContextLayer {
        seed_count: seeds.len(),
        inspected_block_count,
        nodes,
        edges,
        paths: Vec::new(),
        clusters,
        suppressed_hubs: filtered.suppressed_hubs,
    }
}

fn nodes_from_seeds_and_edges(
    seeds: &[FlowContextSeed],
    edges: &[super::model::FlowContextEdge],
) -> Vec<FlowContextNode> {
    let mut nodes = BTreeMap::<String, FlowContextNode>::new();
    for seed in seeds {
        nodes.insert(seed.address.clone(), seed.as_node());
    }
    for edge in edges {
        nodes.entry(edge.source.clone()).or_insert_with(|| {
            FlowContextNode::new(
                &edge.source,
                match edge.kind {
                    FlowContextEdgeKind::SharedFunder => FlowContextNodeRole::Funder,
                    FlowContextEdgeKind::SharedSink => FlowContextNodeRole::Intermediary,
                    _ => FlowContextNodeRole::Unknown,
                },
            )
        });
        nodes.entry(edge.target.clone()).or_insert_with(|| {
            FlowContextNode::new(
                &edge.target,
                match edge.kind {
                    FlowContextEdgeKind::SharedSink => FlowContextNodeRole::Sink,
                    _ => FlowContextNodeRole::Unknown,
                },
            )
        });
    }
    nodes.into_values().collect()
}

fn build_connector_clusters(edges: &[super::model::FlowContextEdge]) -> Vec<FlowContextCluster> {
    let mut members_by_connector =
        BTreeMap::<(FlowContextEdgeKind, String), BTreeSet<String>>::new();
    for edge in edges {
        match edge.kind {
            FlowContextEdgeKind::SharedFunder => {
                members_by_connector
                    .entry((edge.kind.clone(), edge.source.clone()))
                    .or_default()
                    .insert(edge.target.clone());
            }
            FlowContextEdgeKind::SharedSink => {
                members_by_connector
                    .entry((edge.kind.clone(), edge.target.clone()))
                    .or_default()
                    .insert(edge.source.clone());
            }
            _ => {}
        }
    }

    members_by_connector
        .into_iter()
        .filter_map(|((kind, connector), members)| {
            if members.len() < 2 {
                return None;
            }
            let id = format!("{}:{}", kind.stable_key(), connector);
            Some(FlowContextCluster {
                id,
                kind,
                members: members.into_iter().collect(),
                connector: Some(connector),
                confidence: 0.0,
                edge_count: 0,
                explanation: "second-order fund-flow connector links multiple token actors"
                    .to_string(),
            })
        })
        .collect()
}
