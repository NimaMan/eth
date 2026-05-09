use std::collections::HashMap;
use std::sync::Mutex;

use alloy_primitives::Address;

use super::UniswapV2PoolMetadata;

#[derive(Debug, Default)]
pub(crate) struct RethChainMetadataCache {
    v2_pool_metadata: Mutex<HashMap<Address, Option<UniswapV2PoolMetadata>>>,
    token_decimals: Mutex<HashMap<Address, u8>>,
}

impl RethChainMetadataCache {
    pub(crate) fn v2_pool_metadata(
        &self,
        pool_address: Address,
    ) -> Option<Option<UniswapV2PoolMetadata>> {
        self.v2_pool_metadata
            .lock()
            .ok()
            .and_then(|cache| cache.get(&pool_address).cloned())
    }

    pub(crate) fn remember_v2_pool_metadata(
        &self,
        pool_address: Address,
        metadata: Option<UniswapV2PoolMetadata>,
    ) {
        if let Ok(mut cache) = self.v2_pool_metadata.lock() {
            cache.insert(pool_address, metadata);
        }
    }

    pub(crate) fn token_decimals(&self, token_address: Address) -> Option<u8> {
        self.token_decimals
            .lock()
            .ok()
            .and_then(|cache| cache.get(&token_address).copied())
    }

    pub(crate) fn remember_token_decimals(&self, token_address: Address, decimals: u8) {
        if let Ok(mut cache) = self.token_decimals.lock() {
            cache.insert(token_address, decimals);
        }
    }
}
