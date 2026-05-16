pub mod block_plan;
pub mod flow_context;
pub mod summary;
pub mod timeline;
pub mod token_graph;
pub mod token_network_builder;

use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

use eth_token::erc20::ERC20TokenMetadata;
use eyre::{eyre, Result};
use reth_chain_query::RethQueryProvider;
use tx_processor::{BlockProcessor, ProcessedBlockProvider, ProcessedBlockReplayStoreWriter};

use crate::token_analytics::network::adapters::{
    address_index::RethIndexAddressParticipationQuery,
    block_loader::{load_blocks, PreloadedProcessedBlockLoader},
    metadata::load_token_metadata,
};
use crate::token_analytics::network::job::TokenNetworkAnalysisJob;
use crate::token_analytics::network::pipeline::summary::TokenNetworkAnalysisResult;
use crate::token_analytics::network::request::ResolvedTokenNetworkAnalysisRequest;

const TOKEN_METADATA_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct TokenNetworkAnalysisPipeline {
    provider: Arc<RethQueryProvider>,
    replay_store: Option<Arc<ProcessedBlockReplayStoreWriter>>,
}

impl TokenNetworkAnalysisPipeline {
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
        request: ResolvedTokenNetworkAnalysisRequest,
        job: Arc<TokenNetworkAnalysisJob>,
    ) -> Result<TokenNetworkAnalysisResult> {
        let token_address = request.token_address()?;
        let index_query = RethIndexAddressParticipationQuery::new(self.provider.clone());
        let block_provider = ProcessedBlockProvider::new(
            BlockProcessor::new(self.provider.clone()),
            self.provider.clone(),
            self.replay_store.clone(),
        );

        job.mark_running("loading_token_metadata").await;
        let metadata = load_token_metadata_with_timeout(
            self.provider.clone(),
            token_address,
            request.end_block,
        )?;

        let token_network = token_network_builder::TokenNetworkBlockBuilder::new(
            self.provider.clone(),
            &block_provider,
            &index_query,
            job.as_ref(),
            &request,
        )
        .build(metadata)
        .await?;

        job.mark_running("planning_flow_context").await;
        let flow_config = request.flow_context_config();
        let context_blocks = block_plan::context_block_numbers(
            &token_network.token_graph.graph,
            &flow_config,
            &index_query,
        )?;
        let mut all_blocks = token_network.blocks_by_number;
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
            &token_network.token_graph.graph,
        )?;
        let timeline = request.include_timeline.then(|| {
            timeline::build_timeline(
                &token_network.token_graph.graph,
                &request.flow_context_config(),
                &flow_artifacts,
                &token_network.active_blocks,
                &block_timestamps,
            )
        });
        let flow_context = flow_artifacts.layer;

        Ok(TokenNetworkAnalysisResult {
            request,
            token: token_network.token_graph.token,
            graph: token_network.token_graph.summary,
            token_block_count: token_network.active_blocks.len(),
            context_block_count: context_blocks.len(),
            loaded_block_count,
            flow_context,
            timeline,
        })
    }
}

fn load_token_metadata_with_timeout(
    provider: Arc<RethQueryProvider>,
    token_address: alloy_primitives::Address,
    block_number: u64,
) -> Result<ERC20TokenMetadata> {
    let (sender, receiver) = channel();
    std::thread::Builder::new()
        .name(format!(
            "token-network-analysis-token-metadata-{token_address:#x}"
        ))
        .spawn(move || {
            let result = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("token-network-analysis-token-metadata")
                .enable_all()
                .build()
                .map_err(|error| eyre!("failed to build metadata runtime: {error}"))
                .and_then(|runtime| {
                    runtime.block_on(load_token_metadata(
                        provider.as_ref(),
                        token_address,
                        block_number,
                    ))
                });
            let _ = sender.send(result);
        })?;

    match receiver.recv_timeout(TOKEN_METADATA_TIMEOUT) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => Err(eyre!(
            "timed out loading token metadata after {} seconds",
            TOKEN_METADATA_TIMEOUT.as_secs()
        )),
        Err(RecvTimeoutError::Disconnected) => Err(eyre!("token metadata worker disconnected")),
    }
}
