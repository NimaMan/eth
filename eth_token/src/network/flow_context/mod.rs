//! Second-order fund-flow context around token-network actors.

pub mod block_loader;
pub mod builder;
pub mod config;
pub mod extractor;
pub mod hub_filter;
pub mod index_query;
pub mod model;
pub mod promotion;
pub mod scoring;
pub mod seeds;
pub mod snapshot;
pub mod tx_fund_flow;
pub mod windows;

pub use builder::FlowContextBuilder;
pub use config::FlowContextConfig;
pub use extractor::{FlowContextObservation, FlowObservationExtractor};
pub use model::{
    BlockRange, FlowContextCluster, FlowContextEdge, FlowContextEdgeKind, FlowContextEvidence,
    FlowContextHop, FlowContextLayer, FlowContextNode, FlowContextNodeRole, FlowContextPath,
    SuppressedHub,
};
pub use seeds::{select_flow_context_seeds, FlowContextSeed, FlowContextSeedKind};
pub use tx_fund_flow::{TxFundFlowObservationConfig, TxFundFlowObservationExtractor};

#[cfg(test)]
mod tests;
