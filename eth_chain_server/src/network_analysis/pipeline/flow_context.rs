use eth_token::network::flow_context::block_loader::ProcessedBlockLoader;
use eth_token::network::flow_context::index_query::AddressParticipationQuery;
use eth_token::network::flow_context::{
    FlowContextBuildArtifacts, FlowContextBuilder, FlowContextConfig, FlowContextLayer,
    TxFundFlowObservationExtractor,
};
use eth_token::network::graph::RawTokenNetworkGraph;
use eyre::Result;

pub fn build_flow_context<Q, L>(
    config: FlowContextConfig,
    index_query: Q,
    block_loader: L,
    graph: &RawTokenNetworkGraph,
) -> Result<FlowContextLayer>
where
    Q: AddressParticipationQuery,
    L: ProcessedBlockLoader,
{
    FlowContextBuilder::new(
        config,
        index_query,
        block_loader,
        TxFundFlowObservationExtractor::default(),
    )
    .build(graph)
}

pub fn build_flow_context_artifacts<Q, L>(
    config: FlowContextConfig,
    index_query: Q,
    block_loader: L,
    graph: &RawTokenNetworkGraph,
) -> Result<FlowContextBuildArtifacts>
where
    Q: AddressParticipationQuery,
    L: ProcessedBlockLoader,
{
    FlowContextBuilder::new(
        config,
        index_query,
        block_loader,
        TxFundFlowObservationExtractor::default(),
    )
    .build_with_artifacts(graph)
}
