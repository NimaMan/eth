use eth_token::network::flow_context::FlowContextLayer;
use serde::Serialize;

use crate::token_analytics::network::request::ResolvedTokenNetworkAnalysisRequest;

use super::timeline::TokenNetworkAnalysisTimeline;

#[derive(Clone, Debug, Serialize)]
pub struct TokenNetworkAnalysisResult {
    pub request: ResolvedTokenNetworkAnalysisRequest,
    pub token: TokenNetworkAnalysisTokenSummary,
    pub graph: TokenGraphAnalysisSummary,
    pub token_block_count: usize,
    pub context_block_count: usize,
    pub loaded_block_count: usize,
    pub flow_context: FlowContextLayer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<TokenNetworkAnalysisTimeline>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenNetworkAnalysisTokenSummary {
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
