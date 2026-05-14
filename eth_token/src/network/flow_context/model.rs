//! Data model for second-order token fund-flow context.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::network::model::{normalize_network_address, NetworkObservation};

/// Inclusive block range.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BlockRange {
    pub start_block: u64,
    pub end_block: u64,
}

impl BlockRange {
    pub fn new(start_block: u64, end_block: u64) -> Self {
        if start_block <= end_block {
            Self {
                start_block,
                end_block,
            }
        } else {
            Self {
                start_block: end_block,
                end_block: start_block,
            }
        }
    }

    pub fn around(block: u64, lookback: u64, lookahead: u64) -> Self {
        Self {
            start_block: block.saturating_sub(lookback),
            end_block: block.saturating_add(lookahead),
        }
    }

    pub fn contains(&self, block: u64) -> bool {
        self.start_block <= block && block <= self.end_block
    }

    pub fn merge(self, other: Self) -> Self {
        Self {
            start_block: self.start_block.min(other.start_block),
            end_block: self.end_block.max(other.end_block),
        }
    }
}

/// Role of a node inside the second-order layer.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowContextNodeRole {
    TokenSeed,
    Funder,
    Sink,
    Intermediary,
    Hub,
    Unknown,
}

/// Node visible in the fund-flow context layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowContextNode {
    pub address: String,
    pub role: FlowContextNodeRole,
    pub seed: bool,
    pub labels: Vec<String>,
    pub score: f64,
}

impl FlowContextNode {
    pub fn new(address: impl AsRef<str>, role: FlowContextNodeRole) -> Self {
        Self {
            address: normalize_network_address(address),
            role,
            seed: false,
            labels: Vec::new(),
            score: 0.0,
        }
    }

    pub fn seed(address: impl AsRef<str>, score: f64, labels: Vec<String>) -> Self {
        Self {
            address: normalize_network_address(address),
            role: FlowContextNodeRole::TokenSeed,
            seed: true,
            labels,
            score,
        }
    }
}

/// Relationship types in the second-order fund-flow layer.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowContextEdgeKind {
    DirectDenomFlow,
    SharedFunder,
    SharedSink,
    MultiHopFundingPath,
    TemporalFunding,
    ProfitConvergence,
    SuppressedHubLink,
}

impl FlowContextEdgeKind {
    pub fn stable_key(&self) -> &'static str {
        match self {
            Self::DirectDenomFlow => "direct_denom_flow",
            Self::SharedFunder => "shared_funder",
            Self::SharedSink => "shared_sink",
            Self::MultiHopFundingPath => "multi_hop_funding_path",
            Self::TemporalFunding => "temporal_funding",
            Self::ProfitConvergence => "profit_convergence",
            Self::SuppressedHubLink => "suppressed_hub_link",
        }
    }
}

/// Evidence for a background edge. This intentionally mirrors network evidence
/// without forcing every context observation into the raw token graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowContextEvidence {
    pub observation: NetworkObservation,
    pub asset: Option<String>,
    pub symbol: Option<String>,
    pub raw_amount: Option<String>,
    pub scaled_amount: Option<f64>,
    pub description: Option<String>,
    pub attributes: BTreeMap<String, String>,
}

impl FlowContextEvidence {
    pub fn new(observation: NetworkObservation) -> Self {
        Self {
            observation,
            asset: None,
            symbol: None,
            raw_amount: None,
            scaled_amount: None,
            description: None,
            attributes: BTreeMap::new(),
        }
    }
}

/// One hop in a multi-hop context path.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowContextHop {
    pub from: String,
    pub to: String,
    pub evidence: FlowContextEvidence,
}

/// Multi-hop path between two token-network actors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowContextPath {
    pub source: String,
    pub target: String,
    pub hops: Vec<FlowContextHop>,
    pub total_scaled_amount: Option<f64>,
    pub confidence: f64,
}

/// Edge in the second-order layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowContextEdge {
    pub source: String,
    pub target: String,
    pub kind: FlowContextEdgeKind,
    pub weight: u64,
    pub confidence: f64,
    pub total_scaled_amount: Option<f64>,
    pub evidence: Vec<FlowContextEvidence>,
    pub explanation: Option<String>,
}

impl FlowContextEdge {
    pub fn new(
        source: impl AsRef<str>,
        target: impl AsRef<str>,
        kind: FlowContextEdgeKind,
    ) -> Self {
        Self {
            source: normalize_network_address(source),
            target: normalize_network_address(target),
            kind,
            weight: 0,
            confidence: 0.0,
            total_scaled_amount: None,
            evidence: Vec::new(),
            explanation: None,
        }
    }
}

/// Suppressed high-noise node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SuppressedHub {
    pub address: String,
    pub degree: usize,
    pub affected_edge_count: usize,
    pub reason: String,
}

/// Inferred group from second-order evidence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowContextCluster {
    pub id: String,
    pub kind: FlowContextEdgeKind,
    pub members: Vec<String>,
    pub connector: Option<String>,
    pub confidence: f64,
    pub edge_count: usize,
    pub explanation: String,
}

/// Complete second-order layer for one token network.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FlowContextLayer {
    pub seed_count: usize,
    pub inspected_block_count: usize,
    pub nodes: Vec<FlowContextNode>,
    pub edges: Vec<FlowContextEdge>,
    pub paths: Vec<FlowContextPath>,
    pub clusters: Vec<FlowContextCluster>,
    pub suppressed_hubs: Vec<SuppressedHub>,
}
