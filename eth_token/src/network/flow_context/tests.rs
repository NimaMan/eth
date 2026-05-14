use std::collections::BTreeSet;

use eyre::Result;
use tx_processor::ProcessedBlock;

use crate::network::{
    activity::AddressMovement,
    flow_context::{
        block_loader::EmptyProcessedBlockLoader,
        extractor::{
            aggregate_flow_context_edges, FlowContextObservation, FlowObservationExtractor,
        },
        hub_filter::suppress_noisy_hubs,
        index_query::InMemoryAddressParticipation,
        promotion::promote_flow_context_edges,
        snapshot::FlowContextSnapshot,
        FlowContextBuilder, FlowContextConfig,
    },
    graph::RawTokenNetworkGraph,
    ingest::{AddressMovementIngest, NetworkIngestBatch},
    model::{NetworkEdgeKind, NetworkObservation, TokenNetworkId},
};

#[test]
fn aggregates_shared_funder_and_sink_edges() {
    let config = FlowContextConfig::default();
    let seeds = BTreeSet::from([addr(1), addr(2), addr(3)]);
    let funder = addr(9);
    let sink = addr(8);
    let observations = vec![
        flow(&funder, &addr(1), 100, 1.0),
        flow(&funder, &addr(2), 101, 2.0),
        flow(&addr(1), &sink, 120, 0.4),
        flow(&addr(3), &sink, 121, 0.5),
    ];

    let edges = aggregate_flow_context_edges(&observations, &seeds, &config);

    assert!(edges
        .iter()
        .any(|edge| edge.kind == super::model::FlowContextEdgeKind::SharedFunder));
    assert!(edges
        .iter()
        .any(|edge| edge.kind == super::model::FlowContextEdgeKind::SharedSink));
}

#[test]
fn builder_selects_seeds_and_builds_snapshot() {
    let mut graph = graph();
    let mut batch = NetworkIngestBatch::default();
    batch.address_movements.push(AddressMovementIngest::new(
        addr(1),
        AddressMovement::token_in(10.0, NetworkObservation::new(100)),
    ));
    batch.address_movements.push(AddressMovementIngest::new(
        addr(2),
        AddressMovement::token_in(20.0, NetworkObservation::new(101)),
    ));
    graph.apply_batch(batch);

    let mut index = InMemoryAddressParticipation::default();
    index.insert_blocks(addr(1), [90, 100, 110]);
    index.insert_blocks(addr(2), [91, 101, 111]);

    let extractor = StaticExtractor {
        observations: vec![
            flow(&addr(9), &addr(1), 95, 1.0),
            flow(&addr(9), &addr(2), 96, 1.0),
        ],
    };
    let builder = FlowContextBuilder::new(
        FlowContextConfig::default(),
        index,
        EmptyProcessedBlockLoader,
        extractor,
    );

    let layer = builder.build(&graph).expect("flow context builds");
    let snapshot = FlowContextSnapshot::from_layer(&layer);

    assert_eq!(snapshot.summary.seed_count, 2);
    assert!(snapshot.summary.inspected_block_count > 0);
    assert!(snapshot
        .edges
        .iter()
        .any(|edge| edge.kind == super::model::FlowContextEdgeKind::SharedFunder));
}

#[test]
fn hub_filter_suppresses_high_degree_non_seed_nodes() {
    let mut config = FlowContextConfig {
        hub_degree_threshold: 2,
        ..FlowContextConfig::default()
    };
    config.max_evidence_per_edge = 2;
    let seeds = BTreeSet::from([addr(1), addr(2), addr(3)]);
    let observations = vec![
        flow(&addr(9), &addr(1), 100, 1.0),
        flow(&addr(9), &addr(2), 101, 1.0),
        flow(&addr(9), &addr(3), 102, 1.0),
    ];
    let edges = aggregate_flow_context_edges(&observations, &seeds, &config);

    let filtered = suppress_noisy_hubs(edges, &seeds, &config);

    assert!(filtered.edges.is_empty());
    assert_eq!(filtered.suppressed_hubs.len(), 1);
    assert_eq!(filtered.suppressed_hubs[0].address, addr(9));
}

#[test]
fn promotion_maps_shared_funder_to_funding_edge() {
    let config = FlowContextConfig::default();
    let seeds = BTreeSet::from([addr(1), addr(2)]);
    let observations = vec![
        flow(&addr(9), &addr(1), 100, 1.0),
        flow(&addr(9), &addr(2), 101, 1.0),
    ];
    let mut edges = aggregate_flow_context_edges(&observations, &seeds, &config);
    super::scoring::score_edges(&mut edges);

    let promoted = promote_flow_context_edges(&edges, &config);

    assert!(promoted
        .iter()
        .any(|edge| edge.kind == NetworkEdgeKind::Funding));
}

#[derive(Clone, Debug)]
struct StaticExtractor {
    observations: Vec<FlowContextObservation>,
}

impl FlowObservationExtractor for StaticExtractor {
    fn extract_observations(
        &self,
        _blocks: &[ProcessedBlock],
        _seed_addresses: &BTreeSet<String>,
    ) -> Result<Vec<FlowContextObservation>> {
        Ok(self.observations.clone())
    }
}

fn graph() -> RawTokenNetworkGraph {
    RawTokenNetworkGraph::new(TokenNetworkId::new(
        Some(1),
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ))
}

fn flow(from: &str, to: &str, block: u64, amount: f64) -> FlowContextObservation {
    FlowContextObservation::new(from, to, NetworkObservation::new(block)).with_scaled_amount(
        Some("eth"),
        Some("ETH"),
        amount,
    )
}

fn addr(index: u8) -> String {
    format!("0x{index:040x}")
}
