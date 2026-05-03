/// Database initialization logic for TxProcessor
///
/// Handles the complex Reth database setup that was previously in lib.rs
use super::provider_factory::TxProcessorProviderFactory;
use eyre::Result;

/// Create provider factory for direct Reth database access
pub fn create_provider_factory(reth_datadir: &str) -> Result<TxProcessorProviderFactory> {
    Ok((*reth_chain_query::provider_factory_from_datadir(reth_datadir)?).clone())
}
