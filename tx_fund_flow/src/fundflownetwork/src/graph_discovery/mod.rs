//! Graph Discovery Module
//!
//! Discovers the undirected graph structure of fund flows by:
//! - Querying tx_participants to find connections
//! - Retrieving entity types from addresses table
//! - Building preliminary graph for deep analysis
//! - Selecting high-priority transactions

pub mod db_queries;
pub mod explorer;
pub mod routing_rules;
pub mod tx_selector;
pub mod types;

pub use explorer::GraphExplorer;
pub use routing_rules::RoutingRules;
pub use tx_selector::TxSelector;
pub use types::{
    DiscoveryConfig, DiscoveryOutput, DiscoveryStats, GraphEdge, GraphNode, StopReason,
    TxCandidate, UndirectedGraph,
};
