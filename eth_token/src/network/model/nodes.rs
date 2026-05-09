//! Node types and metadata for the token network.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::network::model::{
    evidence::ObservationRange,
    ids::NetworkNodeId,
    labels::{NetworkLabel, NetworkLabelKind, NetworkLabelSource},
};

/// Structural role of a node in the token network.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum NetworkNodeKind {
    Token,
    Address,
    Pool,
    ControlActor,
    LiquidityActor,
    Intermediary,
    TimeWindow,
    Synthetic,
    Unknown,
    Custom(String),
}

impl Default for NetworkNodeKind {
    fn default() -> Self {
        Self::Unknown
    }
}

impl NetworkNodeKind {
    pub fn stable_key(&self) -> String {
        match self {
            Self::Token => "token",
            Self::Address => "address",
            Self::Pool => "pool",
            Self::ControlActor => "control_actor",
            Self::LiquidityActor => "liquidity_actor",
            Self::Intermediary => "intermediary",
            Self::TimeWindow => "time_window",
            Self::Synthetic => "synthetic",
            Self::Unknown => "unknown",
            Self::Custom(value) => value.as_str(),
        }
        .to_string()
    }
}

/// Canonical node state for the raw token network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkNode {
    pub id: NetworkNodeId,
    pub kind: NetworkNodeKind,
    pub labels: Vec<NetworkLabel>,
    pub observed: ObservationRange,
    pub attributes: BTreeMap<String, String>,
}

impl NetworkNode {
    pub fn new(id: NetworkNodeId, kind: NetworkNodeKind) -> Self {
        Self {
            id,
            kind,
            labels: Vec::new(),
            observed: ObservationRange::default(),
            attributes: BTreeMap::new(),
        }
    }

    pub fn token(address: impl AsRef<str>) -> Self {
        let mut node = Self::new(NetworkNodeId::token(address), NetworkNodeKind::Token);
        node.add_observed_label(
            NetworkLabelKind::TokenContract,
            NetworkLabelSource::TokenState,
        );
        node
    }

    pub fn address(address: impl AsRef<str>) -> Self {
        Self::new(NetworkNodeId::address(address), NetworkNodeKind::Address)
    }

    pub fn pool(address: impl AsRef<str>) -> Self {
        let mut node = Self::new(NetworkNodeId::pool_address(address), NetworkNodeKind::Pool);
        node.add_observed_label(NetworkLabelKind::Pool, NetworkLabelSource::PoolState);
        node
    }

    pub fn add_label(&mut self, label: NetworkLabel) {
        self.labels.push(label);
    }

    pub fn add_observed_label(&mut self, kind: NetworkLabelKind, source: NetworkLabelSource) {
        self.add_label(NetworkLabel::observed(kind, source));
    }

    pub fn has_label(&self, kind: &NetworkLabelKind) -> bool {
        self.labels.iter().any(|label| &label.kind == kind)
    }
}
