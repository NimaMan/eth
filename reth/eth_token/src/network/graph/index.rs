//! Lookup indexes for token-network graph state.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::network::model::{NetworkNodeId, NetworkPoolId};

/// Secondary indexes for the raw token-network graph.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct NetworkGraphIndex {
    pub token_nodes: BTreeSet<String>,
    pub address_nodes: BTreeMap<String, String>,
    pub pool_nodes: BTreeMap<String, String>,
    pub time_window_nodes: BTreeSet<String>,
    pub synthetic_nodes: BTreeSet<String>,
}

impl NetworkGraphIndex {
    pub fn index_node(&mut self, node_id: &NetworkNodeId) {
        let stable_key = node_id.stable_key();
        match node_id {
            NetworkNodeId::Token(address) => {
                self.token_nodes.insert(stable_key);
                let _ = address;
            }
            NetworkNodeId::Address(address) => {
                self.address_nodes.insert(address.clone(), stable_key);
            }
            NetworkNodeId::Pool(pool_id) => {
                self.pool_nodes.insert(pool_key(pool_id), stable_key);
            }
            NetworkNodeId::TimeWindow(_) => {
                self.time_window_nodes.insert(stable_key);
            }
            NetworkNodeId::Synthetic(_) => {
                self.synthetic_nodes.insert(stable_key);
            }
        }
    }

    pub fn address_node_key(&self, address: impl AsRef<str>) -> Option<&String> {
        self.address_nodes
            .get(&address.as_ref().trim().to_ascii_lowercase())
    }

    pub fn pool_node_key(&self, pool_id: &NetworkPoolId) -> Option<&String> {
        self.pool_nodes.get(&pool_key(pool_id))
    }
}

fn pool_key(pool_id: &NetworkPoolId) -> String {
    match pool_id {
        NetworkPoolId::Address(address) => address.clone(),
        NetworkPoolId::PoolId(pool_id) => pool_id.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_addresses_and_pools() {
        let mut index = NetworkGraphIndex::default();
        index.index_node(&NetworkNodeId::address("0xabc"));
        index.index_node(&NetworkNodeId::pool_address("0xpool"));

        assert_eq!(
            index.address_node_key("0xABC").map(String::as_str),
            Some("address:0xabc")
        );
        assert_eq!(
            index
                .pool_node_key(&NetworkPoolId::address("0xPOOL"))
                .map(String::as_str),
            Some("pool_address:0xpool")
        );
    }
}
