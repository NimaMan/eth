//! Fund Flow Network Analysis
//!
//! This module provides fund flow network analysis capabilities for TX_FUND_FLOW.
//! It builds interactive, expandable networks showing ETH and token flows between addresses.

pub mod fund_flow_analyzer;
pub mod network_builder;
pub mod network_types;
pub mod state_changes;
// pub mod interactive_builder; // Disabled - needs schema updates
pub mod graph_discovery;
pub mod processed_network_builder;
pub mod tx_processor_integration;
pub mod visualization;

// Re-export main types
pub use fund_flow_analyzer::{FundFlowAnalyzer, NetBalance};
pub use network_builder::NetworkBuilder;
pub use network_types::{
    EdgeType, FundFlowNetwork, NetworkEdge, NetworkNode, NetworkStats, NodeType,
};
pub use state_changes::{AddressStateChange, StateChangeAnalyzer};
// pub use interactive_builder::{InteractiveFundFlowNetwork, InteractiveConfig, ExpansionCandidate}; // Disabled
pub use processed_network_builder::{
    DiscoveredFundFlowNetwork, FundFlowBuildConfig, FundFlowProcessingStats,
    ProcessedFundFlowNetworkBuilder, TransactionProcessingFailure, DEFAULT_MAX_PROCESSED_TXS,
};
pub use tx_processor_integration::{extract_fund_flows_from_processed_tx, ProcessedTxConverter};
pub use visualization::{CytoscapeExporter, GraphMLExporter, VisJsExporter};
