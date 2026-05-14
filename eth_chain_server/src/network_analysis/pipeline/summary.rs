use eth_token::network::flow_context::FlowContextLayer;
use serde::Serialize;

use crate::network_analysis::request::ResolvedNetworkAnalysisRequest;

use super::timeline::NetworkAnalysisTimeline;

#[derive(Clone, Debug, Serialize)]
pub struct NetworkAnalysisResult {
    pub request: ResolvedNetworkAnalysisRequest,
    pub token: NetworkAnalysisTokenSummary,
    pub graph: TokenGraphAnalysisSummary,
    pub token_block_count: usize,
    pub context_block_count: usize,
    pub loaded_block_count: usize,
    pub flow_context: FlowContextLayer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<NetworkAnalysisTimeline>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NetworkAnalysisTokenSummary {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: String,
    pub creation_block: Option<u64>,
    pub creator_address: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenGraphAnalysisSummary {
    pub node_count: usize,
    pub edge_count: usize,
    pub address_count: usize,
    pub applied_batches: u64,
}
