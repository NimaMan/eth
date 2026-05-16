use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use eth_token::erc20::ERC20TokenMetadata;
use eth_token::network::flow_context::index_query::AddressParticipationQuery;
use eth_token::network::flow_context::model::BlockRange;
use eth_token::network::graph::RawTokenNetworkGraph;
use eth_token::network::model::{NetworkNodeId, NetworkPoolId};
use eyre::{bail, Result};
use reth_chain_query::RethQueryProvider;
use tx_processor::{ProcessedBlock, ProcessedBlockProvider};

use crate::token_analytics::network::adapters::block_loader::load_blocks;
use crate::token_analytics::network::job::TokenNetworkAnalysisJob;
use crate::token_analytics::network::request::ResolvedTokenNetworkAnalysisRequest;

use super::token_graph::TokenGraphBuildResult;
use super::{block_plan, token_graph};

pub struct TokenNetworkBlockBuildResult {
    pub token_graph: TokenGraphBuildResult,
    pub blocks_by_number: BTreeMap<u64, ProcessedBlock>,
    pub active_blocks: Vec<u64>,
    pub token_blocks: Vec<u64>,
    pub pool_blocks: Vec<u64>,
}

pub struct TokenNetworkBlockBuilder<'a, Q>
where
    Q: AddressParticipationQuery,
{
    provider: Arc<RethQueryProvider>,
    block_provider: &'a ProcessedBlockProvider,
    index_query: &'a Q,
    job: &'a TokenNetworkAnalysisJob,
    request: &'a ResolvedTokenNetworkAnalysisRequest,
}

impl<'a, Q> TokenNetworkBlockBuilder<'a, Q>
where
    Q: AddressParticipationQuery,
{
    pub fn new(
        provider: Arc<RethQueryProvider>,
        block_provider: &'a ProcessedBlockProvider,
        index_query: &'a Q,
        job: &'a TokenNetworkAnalysisJob,
        request: &'a ResolvedTokenNetworkAnalysisRequest,
    ) -> Self {
        Self {
            provider,
            block_provider,
            index_query,
            job,
            request,
        }
    }

    pub async fn build(
        &self,
        metadata: ERC20TokenMetadata,
    ) -> Result<TokenNetworkBlockBuildResult> {
        self.job.mark_running("planning_token_blocks").await;
        let token_blocks = block_plan::token_participation_blocks(self.index_query, self.request)?;
        if token_blocks.is_empty() {
            bail!(
                "no indexed token participation blocks found for {}; populate reth_index/address_to_blocks first or adjust the requested block range",
                self.request.token
            );
        }
        self.job
            .update_progress(|progress| {
                progress.token_blocks_selected = token_blocks.len();
                progress.blocks_to_load = token_blocks.len();
            })
            .await;

        let mut blocks_by_number = load_blocks(
            self.block_provider,
            &token_blocks,
            self.job,
            "loading_token_blocks",
        )
        .await?;

        let mut token_graph = self
            .build_graph(metadata.clone(), &blocks_by_number)
            .await?;
        let mut seen_pool_addresses = BTreeSet::new();
        let mut pool_blocks = BTreeSet::new();

        loop {
            let new_pool_addresses = pool_addresses_from_graph(&token_graph.graph)
                .into_iter()
                .filter(|address| seen_pool_addresses.insert(address.clone()))
                .collect::<Vec<_>>();
            if new_pool_addresses.is_empty() {
                break;
            }

            self.job.mark_running("planning_pool_blocks").await;
            let discovered_pool_blocks =
                pool_participation_blocks(self.index_query, self.request, &new_pool_addresses)?;
            pool_blocks.extend(discovered_pool_blocks.iter().copied());
            let missing_pool_blocks = discovered_pool_blocks
                .into_iter()
                .filter(|block_number| !blocks_by_number.contains_key(block_number))
                .collect::<Vec<_>>();
            if missing_pool_blocks.is_empty() {
                continue;
            }

            self.job
                .update_progress(|progress| {
                    progress.blocks_to_load += missing_pool_blocks.len();
                })
                .await;
            blocks_by_number.extend(
                load_blocks(
                    self.block_provider,
                    &missing_pool_blocks,
                    self.job,
                    "loading_pool_blocks",
                )
                .await?,
            );
            token_graph = self
                .build_graph(metadata.clone(), &blocks_by_number)
                .await?;
        }

        let active_blocks = merge_active_blocks(&token_blocks, &pool_blocks);
        self.job
            .update_progress(|progress| {
                progress.token_blocks_selected = active_blocks.len();
            })
            .await;

        Ok(TokenNetworkBlockBuildResult {
            token_graph,
            blocks_by_number,
            active_blocks,
            token_blocks,
            pool_blocks: pool_blocks.into_iter().collect(),
        })
    }

    async fn build_graph(
        &self,
        metadata: ERC20TokenMetadata,
        blocks_by_number: &BTreeMap<u64, ProcessedBlock>,
    ) -> Result<TokenGraphBuildResult> {
        self.job.mark_running("building_token_graph").await;
        token_graph::build_token_graph(
            self.provider.clone(),
            metadata,
            blocks_by_number,
            self.request.history_limit,
        )
        .await
    }
}

fn pool_participation_blocks(
    index_query: &impl AddressParticipationQuery,
    request: &ResolvedTokenNetworkAnalysisRequest,
    pool_addresses: &[String],
) -> Result<Vec<u64>> {
    let range = BlockRange::new(request.start_block, request.end_block);
    let mut selected = BTreeSet::new();
    for pool_address in pool_addresses {
        selected.extend(index_query.blocks_for_address(pool_address, range)?);
    }
    Ok(selected.into_iter().collect())
}

fn pool_addresses_from_graph(graph: &RawTokenNetworkGraph) -> Vec<String> {
    let mut addresses = graph
        .nodes
        .values()
        .filter_map(|node| match &node.id {
            NetworkNodeId::Pool(NetworkPoolId::Address(address)) => Some(address.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    addresses.sort();
    addresses.dedup();
    addresses
}

fn merge_active_blocks(token_blocks: &[u64], pool_blocks: &BTreeSet<u64>) -> Vec<u64> {
    token_blocks
        .iter()
        .copied()
        .chain(pool_blocks.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use eth_token::network::flow_context::index_query::AddressParticipationQuery;
    use eth_token::network::flow_context::model::BlockRange;
    use eth_token::network::graph::RawTokenNetworkGraph;
    use eth_token::network::model::{NetworkNodeId, TokenNetworkId};
    use eyre::Result;

    use crate::token_analytics::network::request::ResolvedTokenNetworkAnalysisRequest;

    use super::{merge_active_blocks, pool_addresses_from_graph, pool_participation_blocks};

    #[derive(Default)]
    struct FakeAddressIndex {
        blocks_by_address: BTreeMap<String, Vec<u64>>,
    }

    impl AddressParticipationQuery for FakeAddressIndex {
        fn blocks_for_address(&self, address: &str, range: BlockRange) -> Result<Vec<u64>> {
            Ok(self
                .blocks_by_address
                .get(&address.to_ascii_lowercase())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|block| range.contains(*block))
                .collect())
        }
    }

    fn request() -> ResolvedTokenNetworkAnalysisRequest {
        ResolvedTokenNetworkAnalysisRequest {
            token: "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            start_block: 10,
            end_block: 20,
            history_limit: 100,
            max_token_blocks: 10,
            max_seeds: 4,
            max_blocks_per_address: 4,
            lookback_blocks: 1,
            lookahead_blocks: 1,
            include_timeline: false,
        }
    }

    #[test]
    fn extracts_only_address_based_pool_nodes() {
        let mut graph = RawTokenNetworkGraph::new(TokenNetworkId::new(
            None,
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ));
        graph.ensure_node(NetworkNodeId::pool_address(
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ));
        graph.ensure_node(NetworkNodeId::pool_address(
            "0xcccccccccccccccccccccccccccccccccccccccc",
        ));
        graph.ensure_node(NetworkNodeId::pool(
            eth_token::network::model::NetworkPoolId::pool_id("0xnot_an_address_pool_id"),
        ));

        let addresses = pool_addresses_from_graph(&graph);

        assert_eq!(
            addresses,
            vec![
                "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                "0xcccccccccccccccccccccccccccccccccccccccc".to_string()
            ]
        );
    }

    #[test]
    fn pool_participation_blocks_queries_each_pool_and_dedups() {
        let mut index = FakeAddressIndex::default();
        index.blocks_by_address.insert(
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            vec![9, 10, 11, 20, 21],
        );
        index.blocks_by_address.insert(
            "0xcccccccccccccccccccccccccccccccccccccccc".to_string(),
            vec![11, 12],
        );

        let blocks = pool_participation_blocks(
            &index,
            &request(),
            &[
                "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                "0xcccccccccccccccccccccccccccccccccccccccc".to_string(),
            ],
        )
        .expect("pool blocks resolve");

        assert_eq!(blocks, vec![10, 11, 12, 20]);
    }

    #[test]
    fn merge_active_blocks_sorts_and_dedups_token_and_pool_blocks() {
        let token_blocks = vec![12, 10, 11];
        let pool_blocks = [11, 14, 13].into_iter().collect();

        assert_eq!(
            merge_active_blocks(&token_blocks, &pool_blocks),
            vec![10, 11, 12, 13, 14]
        );
    }
}
