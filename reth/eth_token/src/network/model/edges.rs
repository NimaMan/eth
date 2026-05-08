//! Edge types and metadata for token-network relationships.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::network::model::{
    evidence::{EvidenceSummary, NetworkEvidence},
    ids::{NetworkEdgeId, NetworkNodeId},
    labels::NetworkConfidence,
};

/// Relationship type between two network nodes.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum NetworkEdgeKind {
    TokenTransfer,
    DenomTransfer,
    PoolTrade,
    LpTransfer,
    LpApproval,
    FeeSourceTouches,
    Funding,
    ControlRelation,
    SharedIntermediary,
    TemporalCoactivity,
    PoolCreation,
    LiquidityEvent,
    Synthetic,
    Custom(String),
}

impl NetworkEdgeKind {
    pub fn stable_key(&self) -> String {
        match self {
            Self::TokenTransfer => "token_transfer",
            Self::DenomTransfer => "denom_transfer",
            Self::PoolTrade => "pool_trade",
            Self::LpTransfer => "lp_transfer",
            Self::LpApproval => "lp_approval",
            Self::FeeSourceTouches => "fee_source_touches",
            Self::Funding => "funding",
            Self::ControlRelation => "control_relation",
            Self::SharedIntermediary => "shared_intermediary",
            Self::TemporalCoactivity => "temporal_coactivity",
            Self::PoolCreation => "pool_creation",
            Self::LiquidityEvent => "liquidity_event",
            Self::Synthetic => "synthetic",
            Self::Custom(value) => value.as_str(),
        }
        .to_string()
    }
}

/// Direction semantics for a collapsed edge.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkEdgeDirection {
    Directed,
    Undirected,
}

impl Default for NetworkEdgeDirection {
    fn default() -> Self {
        Self::Directed
    }
}

/// Amount totals attached to a collapsed edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkEdgeAmountSummary {
    pub asset: Option<String>,
    pub symbol: Option<String>,
    pub raw_total: Option<String>,
    pub scaled_total: Option<f64>,
}

impl NetworkEdgeAmountSummary {
    pub fn new(asset: Option<impl Into<String>>, symbol: Option<impl Into<String>>) -> Self {
        Self {
            asset: asset.map(Into::into),
            symbol: symbol.map(Into::into),
            raw_total: None,
            scaled_total: None,
        }
    }
}

/// Canonical collapsed edge state for the token network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkEdge {
    pub id: NetworkEdgeId,
    pub kind: NetworkEdgeKind,
    pub direction: NetworkEdgeDirection,
    pub evidence: EvidenceSummary,
    pub amount: Option<NetworkEdgeAmountSummary>,
    pub confidence: Option<NetworkConfidence>,
    pub attributes: BTreeMap<String, String>,
}

impl NetworkEdge {
    pub fn new(source: NetworkNodeId, target: NetworkNodeId, kind: NetworkEdgeKind) -> Self {
        let id = NetworkEdgeId::new(source, target, kind.stable_key());
        Self {
            id,
            kind,
            direction: NetworkEdgeDirection::Directed,
            evidence: EvidenceSummary::default(),
            amount: None,
            confidence: None,
            attributes: BTreeMap::new(),
        }
    }

    pub fn undirected(mut self) -> Self {
        self.direction = NetworkEdgeDirection::Undirected;
        self
    }

    pub fn record_evidence(&mut self, evidence: NetworkEvidence) {
        self.evidence.record(evidence);
    }

    pub fn source(&self) -> &NetworkNodeId {
        &self.id.source
    }

    pub fn target(&self) -> &NetworkNodeId {
        &self.id.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::model::ids::NetworkNodeId;

    #[test]
    fn edge_kind_builds_stable_id() {
        let edge = NetworkEdge::new(
            NetworkNodeId::address("0xA"),
            NetworkNodeId::pool_address("0xP"),
            NetworkEdgeKind::PoolTrade,
        );

        assert_eq!(
            edge.id.stable_key(),
            "address:0xa->pool_address:0xp:pool_trade"
        );
    }
}
