/// Provider factory management utilities
///
/// Helper functions for managing Reth provider factory instances
use std::sync::Arc;

pub type TxProcessorProviderFactory = reth_chain_query::provider::RethProviderFactory;

/// Convert provider factory to Arc for sharing across components
pub fn arc_provider_factory(
    provider_factory: TxProcessorProviderFactory,
) -> Arc<TxProcessorProviderFactory> {
    Arc::new(provider_factory)
}
