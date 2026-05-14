//! Fund-flow observation extraction and aggregation.

use std::collections::{BTreeMap, BTreeSet};

use eyre::Result;
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedBlock;

use crate::network::model::{normalize_network_address, NetworkObservation};

use super::{
    config::FlowContextConfig,
    model::{FlowContextEdge, FlowContextEdgeKind, FlowContextEvidence},
};

/// A normalized non-token fund-flow observation relevant to token-network seeds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowContextObservation {
    pub from: String,
    pub to: String,
    pub asset: Option<String>,
    pub symbol: Option<String>,
    pub raw_amount: Option<String>,
    pub scaled_amount: Option<f64>,
    pub observation: NetworkObservation,
}

impl FlowContextObservation {
    pub fn new(
        from: impl AsRef<str>,
        to: impl AsRef<str>,
        observation: NetworkObservation,
    ) -> Self {
        Self {
            from: normalize_network_address(from),
            to: normalize_network_address(to),
            asset: None,
            symbol: None,
            raw_amount: None,
            scaled_amount: None,
            observation,
        }
    }

    pub fn with_scaled_amount(
        mut self,
        asset: Option<impl Into<String>>,
        symbol: Option<impl Into<String>>,
        amount: f64,
    ) -> Self {
        self.asset = asset.map(Into::into);
        self.symbol = symbol.map(Into::into);
        self.scaled_amount = amount.is_finite().then_some(amount);
        self
    }

    pub fn with_amount(
        mut self,
        asset: Option<String>,
        symbol: Option<String>,
        raw_amount: Option<String>,
        scaled_amount: Option<f64>,
    ) -> Self {
        self.asset = asset;
        self.symbol = symbol;
        self.raw_amount = raw_amount;
        self.scaled_amount = scaled_amount.filter(|amount| amount.is_finite());
        self
    }

    pub fn evidence(&self, description: impl Into<String>) -> FlowContextEvidence {
        let mut evidence = FlowContextEvidence::new(self.observation.clone());
        evidence.asset = self.asset.clone();
        evidence.symbol = self.symbol.clone();
        evidence.raw_amount = self.raw_amount.clone();
        evidence.scaled_amount = self.scaled_amount;
        evidence.description = Some(description.into());
        evidence
    }
}

/// Adapter around existing processed-block and `tx_fund_flow` extraction.
pub trait FlowObservationExtractor {
    fn extract_observations(
        &self,
        blocks: &[ProcessedBlock],
        seed_addresses: &BTreeSet<String>,
    ) -> Result<Vec<FlowContextObservation>>;
}

/// No-op extractor used by the scaffold and dry runs.
#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyFlowObservationExtractor;

impl FlowObservationExtractor for EmptyFlowObservationExtractor {
    fn extract_observations(
        &self,
        _blocks: &[ProcessedBlock],
        _seed_addresses: &BTreeSet<String>,
    ) -> Result<Vec<FlowContextObservation>> {
        Ok(Vec::new())
    }
}

/// Convert normalized observations into direct and shared-context edge
/// candidates. This is intentionally independent from block loading so tests can
/// exercise the graph semantics without a chain database.
pub fn aggregate_flow_context_edges(
    observations: &[FlowContextObservation],
    seed_addresses: &BTreeSet<String>,
    config: &FlowContextConfig,
) -> Vec<FlowContextEdge> {
    let observations = observations
        .iter()
        .filter(|observation| {
            observation
                .scaled_amount
                .map(|amount| amount.abs() >= config.min_scaled_amount)
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    let mut edges = BTreeMap::<(String, String, FlowContextEdgeKind), FlowContextEdge>::new();
    for observation in &observations {
        if seed_addresses.contains(&observation.from) || seed_addresses.contains(&observation.to) {
            merge_observation_edge(
                &mut edges,
                &observation.from,
                &observation.to,
                FlowContextEdgeKind::DirectDenomFlow,
                observation.evidence("direct non-token value flow near token activity"),
                config.max_evidence_per_edge,
            );
        }
    }

    let mut recipients_by_funder = BTreeMap::<String, BTreeSet<String>>::new();
    let mut senders_by_sink = BTreeMap::<String, BTreeSet<String>>::new();
    for observation in &observations {
        if seed_addresses.contains(&observation.to) && !seed_addresses.contains(&observation.from) {
            recipients_by_funder
                .entry(observation.from.clone())
                .or_default()
                .insert(observation.to.clone());
        }
        if seed_addresses.contains(&observation.from) && !seed_addresses.contains(&observation.to) {
            senders_by_sink
                .entry(observation.to.clone())
                .or_default()
                .insert(observation.from.clone());
        }
    }

    for observation in &observations {
        if recipients_by_funder
            .get(&observation.from)
            .map(|recipients| recipients.len() >= 2 && seed_addresses.contains(&observation.to))
            .unwrap_or(false)
        {
            merge_observation_edge(
                &mut edges,
                &observation.from,
                &observation.to,
                FlowContextEdgeKind::SharedFunder,
                observation.evidence("same non-token funder reached multiple token actors"),
                config.max_evidence_per_edge,
            );
        }

        if senders_by_sink
            .get(&observation.to)
            .map(|senders| senders.len() >= 2 && seed_addresses.contains(&observation.from))
            .unwrap_or(false)
        {
            merge_observation_edge(
                &mut edges,
                &observation.from,
                &observation.to,
                FlowContextEdgeKind::SharedSink,
                observation.evidence("multiple token actors sent non-token value to the same sink"),
                config.max_evidence_per_edge,
            );
        }
    }

    let mut edges = edges.into_values().collect::<Vec<_>>();
    edges.sort_by(|left, right| {
        right
            .weight
            .cmp(&left.weight)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.target.cmp(&right.target))
    });
    edges.truncate(config.max_context_edges);
    edges
}

fn merge_observation_edge(
    edges: &mut BTreeMap<(String, String, FlowContextEdgeKind), FlowContextEdge>,
    source: &str,
    target: &str,
    kind: FlowContextEdgeKind,
    evidence: FlowContextEvidence,
    max_evidence: usize,
) {
    let key = (source.to_string(), target.to_string(), kind.clone());
    let edge = edges
        .entry(key)
        .or_insert_with(|| FlowContextEdge::new(source, target, kind));
    edge.weight += 1;
    if let Some(amount) = evidence.scaled_amount {
        edge.total_scaled_amount = Some(edge.total_scaled_amount.unwrap_or(0.0) + amount);
    }
    if edge.evidence.len() < max_evidence {
        edge.evidence.push(evidence);
    }
}
