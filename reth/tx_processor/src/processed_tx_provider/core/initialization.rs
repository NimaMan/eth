/// Database initialization logic for TxProcessor
///
/// Handles the complex Reth database setup that was previously in lib.rs
use eyre::Result;
use reth_chainspec::ChainSpecBuilder;
use reth_db::{mdbx::DatabaseArguments, open_db_read_only, ClientVersion};
use reth_node_ethereum::EthereumNode;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_provider::providers::StaticFileProvider;
use reth_provider::ProviderFactory;
use std::path::Path;
use std::sync::Arc;

/// Create provider factory for direct Reth database access
pub fn create_provider_factory(
    reth_datadir: &str,
) -> Result<ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<reth_db::DatabaseEnv>>>> {
    let db_path = Path::new(reth_datadir).join("db");
    let static_files_path = Path::new(reth_datadir).join("static_files");

    let db = Arc::new(open_db_read_only(
        &db_path,
        DatabaseArguments::new(ClientVersion::default()),
    )?);

    let chain_spec = Arc::new(ChainSpecBuilder::mainnet().build());

    let provider_factory = ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<_>>>::new(
        db.clone(),
        chain_spec.clone(),
        StaticFileProvider::read_only(static_files_path, false)?, // Don't watch files - Reth is already watching
    );

    Ok(provider_factory)
}
