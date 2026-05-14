use std::collections::BTreeSet;

use eth_token::network::flow_context::index_query::AddressParticipationQuery;
use eth_token::network::flow_context::model::BlockRange;
use eth_token::network::flow_context::windows::build_flow_context_windows;
use eth_token::network::flow_context::{select_flow_context_seeds, FlowContextConfig};
use eth_token::network::graph::RawTokenNetworkGraph;
use eyre::Result;

use crate::network_analysis::request::ResolvedNetworkAnalysisRequest;

pub fn token_participation_blocks(
    index_query: &impl AddressParticipationQuery,
    request: &ResolvedNetworkAnalysisRequest,
) -> Result<Vec<u64>> {
    let range = BlockRange::new(request.start_block, request.end_block);
    let mut blocks = index_query.blocks_for_address(&request.token, range)?;
    blocks.truncate(request.max_token_blocks);
    Ok(blocks)
}

pub fn context_block_numbers(
    graph: &RawTokenNetworkGraph,
    config: &FlowContextConfig,
    index_query: &impl AddressParticipationQuery,
) -> Result<Vec<u64>> {
    let seeds = select_flow_context_seeds(graph, config);
    let windows = build_flow_context_windows(graph, &seeds, config);
    let mut selected = BTreeSet::new();

    for window in windows {
        let mut blocks = index_query.blocks_for_address(&window.address, window.range)?;
        blocks.truncate(config.max_blocks_per_address);
        selected.extend(blocks);
    }

    Ok(selected.into_iter().collect())
}
