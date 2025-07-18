//! Graph Discovery Module
//! 
//! Discovers the undirected graph structure of fund flows by:
//! - Querying tx_participants to find connections
//! - Retrieving entity types from addresses table
//! - Building preliminary graph for deep analysis
//! - Selecting high-priority transactions

pub mod types;
pub mod db_queries;
pub mod routing_rules;
pub mod tx_selector;
pub mod explorer;

pub use types::{
    PreliminaryGraph, GraphNode, GraphEdge, 
    DiscoveryConfig, DiscoveryOutput, TxCandidate
};
pub use explorer::GraphExplorer;
pub use routing_rules::RoutingRules;
pub use tx_selector::TxSelector;