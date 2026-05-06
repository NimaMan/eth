use std::sync::Arc;

use eyre::{eyre, Result};
use reth_chain_query::RethQueryProvider;

use crate::config::TokenServerConfig;
use crate::runs::{RunManager, TokenProcessedBlockCacheStore};

#[derive(Clone)]
pub struct ServerState {
    pub config: TokenServerConfig,
    pub runs: RunManager,
}

impl ServerState {
    pub fn new(config: TokenServerConfig) -> Result<Self> {
        let datadir = config
            .reth_datadir
            .to_str()
            .ok_or_else(|| eyre!("RETH_DATADIR is not valid UTF-8"))?;
        let provider = Arc::new(RethQueryProvider::new(datadir)?);
        let processed_block_cache = match config.processed_block_cache_dir.as_ref() {
            Some(path) => Some(Arc::new(TokenProcessedBlockCacheStore::open(path)?)),
            None => None,
        };
        let runs = RunManager::new(config.clone(), provider, processed_block_cache);

        Ok(Self { config, runs })
    }
}
