/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub reth_db_path: String,
}

impl DatabaseConfig {
    pub fn new(reth_db_path: &str) -> Self {
        Self {
            reth_db_path: reth_db_path.to_string(),
        }
    }

    pub fn default_mainnet() -> Self {
        Self {
            reth_db_path: "/home/nima/.local/share/reth/mainnet".to_string(),
        }
    }
}

pub type EthPriceProviderFactory = reth_chain_query::provider::RethProviderFactory;

pub fn open_provider_factory(
    reth_db_path: &str,
) -> Result<std::sync::Arc<EthPriceProviderFactory>, crate::core::PriceError> {
    reth_chain_query::provider_factory_from_datadir(reth_db_path)
        .map_err(|error| crate::core::PriceError::DatabaseError(error.to_string()))
}
