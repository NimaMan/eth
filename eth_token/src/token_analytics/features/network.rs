use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenNetworkFeatures {
    pub unique_address_count: Option<u32>,
    pub new_address_count_in_block: Option<u32>,
    pub pool_recycling_transfer_count: Option<u32>,
    pub one_to_many_transfer_count: Option<u32>,
    pub many_to_one_transfer_count: Option<u32>,
    pub creator_centrality: Option<f64>,
    pub owner_centrality: Option<f64>,
    pub node_count: Option<u32>,
    pub edge_count: Option<u32>,
    pub largest_non_protocol_cluster_size: Option<u32>,
    pub shared_non_protocol_funder_count: Option<u32>,
    pub feature_scope: Option<String>,
}
