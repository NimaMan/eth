//! Stable identifiers for token-network nodes and edges.

use serde::{Deserialize, Serialize};

/// Stable identifier for one token network on one chain.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TokenNetworkId {
    pub chain_id: Option<u64>,
    pub token_address: String,
}

impl TokenNetworkId {
    pub fn new(chain_id: Option<u64>, token_address: impl AsRef<str>) -> Self {
        Self {
            chain_id,
            token_address: normalize_network_address(token_address),
        }
    }
}

/// Pool identity that supports both address-based pools and V4-style pool IDs.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum NetworkPoolId {
    Address(String),
    PoolId(String),
}

impl NetworkPoolId {
    pub fn address(address: impl AsRef<str>) -> Self {
        Self::Address(normalize_network_address(address))
    }

    pub fn pool_id(pool_id: impl AsRef<str>) -> Self {
        Self::PoolId(normalize_network_key(pool_id))
    }

    pub fn stable_key(&self) -> String {
        match self {
            Self::Address(address) => format!("pool_address:{address}"),
            Self::PoolId(pool_id) => format!("pool_id:{pool_id}"),
        }
    }
}

/// Synthetic time-window identity used for coactivity around noisy hubs.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NetworkTimeWindowId {
    pub hub: Box<NetworkNodeId>,
    pub start_block: u64,
    pub end_block: u64,
    pub bucket: Option<String>,
}

impl NetworkTimeWindowId {
    pub fn new(
        hub: NetworkNodeId,
        start_block: u64,
        end_block: u64,
        bucket: Option<impl Into<String>>,
    ) -> Self {
        Self {
            hub: Box::new(hub),
            start_block,
            end_block,
            bucket: bucket.map(Into::into),
        }
    }

    pub fn stable_key(&self) -> String {
        let bucket = self.bucket.as_deref().unwrap_or("default");
        format!(
            "time_window:{}:{}:{}:{}",
            self.hub.stable_key(),
            self.start_block,
            self.end_block,
            bucket
        )
    }
}

/// Stable identity for graph nodes.
///
/// Address roles are labels, not identity. For example, a creator, owner, and
/// funder can all point at the same `Address` node with different labels.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum NetworkNodeId {
    Token(String),
    Address(String),
    Pool(NetworkPoolId),
    TimeWindow(NetworkTimeWindowId),
    Synthetic(String),
}

impl NetworkNodeId {
    pub fn token(address: impl AsRef<str>) -> Self {
        Self::Token(normalize_network_address(address))
    }

    pub fn address(address: impl AsRef<str>) -> Self {
        Self::Address(normalize_network_address(address))
    }

    pub fn pool(pool_id: NetworkPoolId) -> Self {
        Self::Pool(pool_id)
    }

    pub fn pool_address(address: impl AsRef<str>) -> Self {
        Self::Pool(NetworkPoolId::address(address))
    }

    pub fn time_window(id: NetworkTimeWindowId) -> Self {
        Self::TimeWindow(id)
    }

    pub fn synthetic(id: impl AsRef<str>) -> Self {
        Self::Synthetic(normalize_network_key(id))
    }

    pub fn stable_key(&self) -> String {
        match self {
            Self::Token(address) => format!("token:{address}"),
            Self::Address(address) => format!("address:{address}"),
            Self::Pool(pool_id) => pool_id.stable_key(),
            Self::TimeWindow(window) => window.stable_key(),
            Self::Synthetic(id) => format!("synthetic:{id}"),
        }
    }
}

/// Stable identity for a collapsed relationship between two graph nodes.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NetworkEdgeId {
    pub source: NetworkNodeId,
    pub target: NetworkNodeId,
    pub relationship: String,
}

impl NetworkEdgeId {
    pub fn new(
        source: NetworkNodeId,
        target: NetworkNodeId,
        relationship: impl AsRef<str>,
    ) -> Self {
        Self {
            source,
            target,
            relationship: normalize_network_key(relationship),
        }
    }

    pub fn stable_key(&self) -> String {
        format!(
            "{}->{}:{}",
            self.source.stable_key(),
            self.target.stable_key(),
            self.relationship
        )
    }
}

pub fn normalize_network_address(value: impl AsRef<str>) -> String {
    value.as_ref().trim().to_ascii_lowercase()
}

pub fn normalize_network_key(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_whitespace() {
                '_'
            } else {
                ch.to_ascii_lowercase()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_ids_are_normalized() {
        assert_eq!(
            NetworkNodeId::address(" 0xABCDEF ").stable_key(),
            "address:0xabcdef"
        );
        assert_eq!(
            NetworkPoolId::address(" 0xPOOL ").stable_key(),
            "pool_address:0xpool"
        );
    }

    #[test]
    fn edge_id_has_deterministic_key() {
        let id = NetworkEdgeId::new(
            NetworkNodeId::address("0xA"),
            NetworkNodeId::address("0xB"),
            "Token Transfer",
        );

        assert_eq!(id.stable_key(), "address:0xa->address:0xb:token_transfer");
    }
}
