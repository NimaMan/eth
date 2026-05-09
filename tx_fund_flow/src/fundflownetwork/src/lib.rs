//! Fund Flow Network Analysis
//! 
//! This module provides fund flow network analysis capabilities for TX_FUND_FLOW.
//! It builds interactive, expandable networks showing ETH and token flows between addresses.

pub mod fund_flow_analyzer;
pub mod state_changes;
pub mod network_types;
pub mod network_builder;
// pub mod interactive_builder; // Disabled - needs schema updates
pub mod visualization;
pub mod tx_processor_integration;
pub mod graph_discovery;
pub mod processed_network_builder;

// Re-export main types
pub use fund_flow_analyzer::{FundFlowAnalyzer, NetBalance};
pub use state_changes::{StateChangeAnalyzer, AddressStateChange};
pub use network_types::{FundFlowNetwork, NetworkNode, NetworkEdge, NodeType, EdgeType, NetworkStats};
pub use network_builder::NetworkBuilder;
// pub use interactive_builder::{InteractiveFundFlowNetwork, InteractiveConfig, ExpansionCandidate}; // Disabled
pub use visualization::{CytoscapeExporter, VisJsExporter, GraphMLExporter};
pub use tx_processor_integration::{extract_fund_flows_from_processed_tx, ProcessedTxConverter};
pub use processed_network_builder::{
    DiscoveredFundFlowNetwork, FundFlowBuildConfig, FundFlowProcessingStats,
    ProcessedFundFlowNetworkBuilder, TransactionProcessingFailure,
    DEFAULT_MAX_PROCESSED_TXS,
};
