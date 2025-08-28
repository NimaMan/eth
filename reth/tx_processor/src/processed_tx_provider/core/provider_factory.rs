/// Provider factory management utilities
/// 
/// Helper functions for managing Reth provider factory instances

use reth_provider::ProviderFactory;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_node_ethereum::EthereumNode;
use std::sync::Arc;

pub type TxProcessorProviderFactory = ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<reth_db::DatabaseEnv>>>;

/// Convert provider factory to Arc for sharing across components
pub fn arc_provider_factory(
    provider_factory: TxProcessorProviderFactory
) -> Arc<TxProcessorProviderFactory> {
    Arc::new(provider_factory)
}