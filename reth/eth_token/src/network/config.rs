//! Configuration for token-network construction and simplification.

use serde::{Deserialize, Serialize};

use crate::network::activity::DEFAULT_ADDRESS_ACTIVITY_HISTORY_LIMIT;

pub const DEFAULT_EDGE_EVIDENCE_LIMIT: usize = 8;

/// Configuration for the raw token-network graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenNetworkGraphConfig {
    pub address_activity_history_limit: usize,
    pub edge_evidence_limit: usize,
}

impl Default for TokenNetworkGraphConfig {
    fn default() -> Self {
        Self {
            address_activity_history_limit: DEFAULT_ADDRESS_ACTIVITY_HISTORY_LIMIT,
            edge_evidence_limit: DEFAULT_EDGE_EVIDENCE_LIMIT,
        }
    }
}
