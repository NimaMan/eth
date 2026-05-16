use std::collections::BTreeMap;
use std::sync::Arc;

use eth_token::chain_metadata::RethChainMetadataProvider;
use eth_token::erc20::ERC20TokenMetadata;
use eth_token::network::graph::RawTokenNetworkGraph;
use eth_token::network::model::normalize_network_address;
use eth_token::tracking::{BlockTokenProcessor, TokenRegistry};
use eyre::{eyre, Result};
use reth_chain_query::RethQueryProvider;
use tx_processor::{PoolBuySellSimulator, ProcessedBlock};

use super::summary::{TokenGraphAnalysisSummary, TokenNetworkAnalysisTokenSummary};

pub struct TokenGraphBuildResult {
    pub token: TokenNetworkAnalysisTokenSummary,
    pub graph: RawTokenNetworkGraph,
    pub summary: TokenGraphAnalysisSummary,
}

pub async fn build_token_graph(
    provider: Arc<RethQueryProvider>,
    metadata: ERC20TokenMetadata,
    blocks_by_number: &BTreeMap<u64, ProcessedBlock>,
    history_limit: usize,
) -> Result<TokenGraphBuildResult> {
    let token_address = normalize_network_address(&metadata.address);
    let discovery_provider = RethChainMetadataProvider::new(provider.as_ref());
    let pool_simulator = PoolBuySellSimulator::from_simulator(provider.simulator().clone());
    let mut registry = TokenRegistry::new();
    registry.add_token(metadata);
    let mut processor = BlockTokenProcessor::with_registry(registry, history_limit);

    for block in blocks_by_number.values() {
        processor
            .process_block_with_discovery_provider(block, &discovery_provider, &pool_simulator)
            .await;
    }

    let token = processor
        .registry
        .token(&token_address)
        .cloned()
        .ok_or_else(|| eyre!("token {token_address} missing after graph build"))?;
    let graph = processor
        .network_graphs
        .get(&token_address)
        .cloned()
        .ok_or_else(|| eyre!("token graph for {token_address} is empty after replay"))?;

    let summary = TokenGraphAnalysisSummary {
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
        address_count: graph.address_activity.len(),
        applied_batches: graph.applied_batches,
    };
    let token = TokenNetworkAnalysisTokenSummary {
        address: token.contract_address,
        name: token.name,
        symbol: token.symbol,
        decimals: token.decimals,
        total_supply: token.total_supply,
        creation_block: token.creation_block,
        creator_address: token.creator_address,
    };

    Ok(TokenGraphBuildResult {
        token,
        graph,
        summary,
    })
}
