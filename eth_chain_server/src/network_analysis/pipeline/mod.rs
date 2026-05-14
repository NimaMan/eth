pub mod block_plan;
pub mod flow_context;
pub mod summary;
pub mod timeline;
pub mod token_graph;

use std::sync::Arc;

use eyre::{bail, Result};
use reth_chain_query::RethQueryProvider;
use tx_processor::{BlockProcessor, ProcessedBlockProvider, ProcessedBlockReplayStoreWriter};

use crate::network_analysis::adapters::{
    address_index::RethIndexAddressParticipationQuery,
    block_loader::{load_blocks, PreloadedProcessedBlockLoader},
    metadata::load_token_metadata,
};
use crate::network_analysis::job::NetworkAnalysisJob;
use crate::network_analysis::pipeline::summary::NetworkAnalysisResult;
use crate::network_analysis::request::ResolvedNetworkAnalysisRequest;

#[derive(Clone)]
pub struct NetworkAnalysisPipeline {
    provider: Arc<RethQueryProvider>,
    replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
}

impl NetworkAnalysisPipeline {
    pub fn new(
        provider: Arc<RethQueryProvider>,
        replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
    ) -> Self {
        Self {
            provider,
            replay_store,
        }
    }

    pub async fn run(
        &self,
        request: ResolvedNetworkAnalysisRequest,
        job: Arc<NetworkAnalysisJob>,
    ) -> Result<NetworkAnalysisResult> {
        let token_address = request.token_address()?;
        let index_query = RethIndexAddressParticipationQuery::new(self.provider.clone());
        let block_provider = ProcessedBlockProvider::new(
            BlockProcessor::new(self.provider.clone()),
            self.provider.clone(),
            self.replay_store.clone(),
        );

        job.mark_running("loading_token_metadata").await;
        let metadata =
            load_token_metadata(self.provider.as_ref(), token_address, request.end_block).await?;

        job.mark_running("planning_token_blocks").await;
        let token_blocks = block_plan::token_participation_blocks(&index_query, &request)?;
        if token_blocks.is_empty() {
            bail!(
                "no indexed token participation blocks found for {}; populate reth_index/address_to_blocks first or adjust the requested block range",
                request.token
            );
        }
        job.update_progress(|progress| {
            progress.token_blocks_selected = token_blocks.len();
            progress.blocks_to_load = token_blocks.len();
        })
        .await;

        let token_blocks_by_number = load_blocks(
            &block_provider,
            &token_blocks,
            job.as_ref(),
            "loading_token_blocks",
        )
        .await?;

        job.mark_running("building_token_graph").await;
        let token_graph = token_graph::build_token_graph(
            self.provider.clone(),
            metadata,
            &token_blocks_by_number,
            request.history_limit,
        )
        .await?;

        job.mark_running("planning_flow_context").await;
        let flow_config = request.flow_context_config();
        let context_blocks =
            block_plan::context_block_numbers(&token_graph.graph, &flow_config, &index_query)?;
        let mut all_blocks = token_blocks_by_number;
        let missing_context_blocks = context_blocks
            .iter()
            .copied()
            .filter(|block_number| !all_blocks.contains_key(block_number))
            .collect::<Vec<_>>();
        job.update_progress(|progress| {
            progress.context_blocks_selected = context_blocks.len();
            progress.blocks_to_load += missing_context_blocks.len();
        })
        .await;

        all_blocks.extend(
            load_blocks(
                &block_provider,
                &missing_context_blocks,
                job.as_ref(),
                "loading_flow_context_blocks",
            )
            .await?,
        );

        job.mark_running("building_flow_context").await;
        let loaded_block_count = all_blocks.len();
        let block_timestamps = all_blocks
            .iter()
            .map(|(block_number, block)| (*block_number, block.header.timestamp))
            .collect();
        let flow_artifacts = flow_context::build_flow_context_artifacts(
            flow_config,
            index_query,
            PreloadedProcessedBlockLoader::new(all_blocks),
            &token_graph.graph,
        )?;
        let timeline = request.include_timeline.then(|| {
            timeline::build_timeline(
                &token_graph.graph,
                &request.flow_context_config(),
                &flow_artifacts,
                &token_blocks,
                &block_timestamps,
            )
        });
        let flow_context = flow_artifacts.layer;

        Ok(NetworkAnalysisResult {
            request,
            token: token_graph.token,
            graph: token_graph.summary,
            token_block_count: token_blocks.len(),
            context_block_count: context_blocks.len(),
            loaded_block_count,
            flow_context,
            timeline,
        })
    }
}
