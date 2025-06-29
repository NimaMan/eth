//! Uniswap V2 protocol implementation

use crate::common::{Result, TokenAddress};
use crate::protocols::traits::{DexPool, DexProtocol, ProtocolFeature};
use async_trait::async_trait;
use ethers::prelude::*;
use std::sync::Arc;
use tracing::{debug, instrument};

use super::{addresses, pool::UniswapV2Pool};

/// Uniswap V2 protocol implementation
pub struct UniswapV2Protocol {
    provider: Arc<Provider<Http>>,
}

impl UniswapV2Protocol {
    /// Create new Uniswap V2 protocol instance
    pub fn new(provider: Arc<Provider<Http>>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl DexProtocol for UniswapV2Protocol {
    fn name(&self) -> &'static str {
        "UniswapV2"
    }
    
    fn version(&self) -> &'static str {
        "2.0"
    }
    
    fn factory_address(&self) -> Address {
        *addresses::FACTORY
    }
    
    fn router_address(&self) -> Address {
        *addresses::ROUTER
    }
    
    #[instrument(skip(self))]
    async fn find_pool(
        &self,
        token_a: TokenAddress,
        token_b: TokenAddress,
    ) -> Result<Option<Box<dyn DexPool>>> {
        let pool_address = super::calculate_pair_address(token_a, token_b);
        
        debug!(
            "Looking for UniswapV2 pool for {:?}/{:?} at {:?}",
            token_a, token_b, pool_address
        );
        
        // Check if pool exists by getting code
        let code = self.provider.get_code(pool_address, None).await?;
        
        if code.is_empty() {
            debug!("No UniswapV2 pool found at {:?}", pool_address);
            return Ok(None);
        }
        
        // Create pool instance
        let pool = UniswapV2Pool::new(
            pool_address,
            token_a,
            token_b,
            self.provider.clone(),
        );
        
        // Verify it's a valid pool by getting reserves
        match pool.get_reserves().await {
            Ok(_) => {
                debug!("Found valid UniswapV2 pool at {:?}", pool_address);
                Ok(Some(Box::new(pool)))
            }
            Err(e) => {
                debug!("Pool at {:?} failed validation: {}", pool_address, e);
                Ok(None)
            }
        }
    }
    
    async fn get_active_pools(&self, _limit: usize) -> Result<Vec<Box<dyn DexPool>>> {
        // For V2, we don't have an easy way to enumerate all pools
        // In production, this would query events or use a subgraph
        tracing::warn!("get_active_pools not fully implemented for UniswapV2");
        Ok(Vec::new())
    }
    
    fn supports_feature(&self, feature: ProtocolFeature) -> bool {
        match feature {
            ProtocolFeature::MultipleFees => false, // V2 has fixed 0.3% fee
            ProtocolFeature::ConcentratedLiquidity => false,
            ProtocolFeature::FlashLoans => false, // V2 doesn't have built-in flash loans
            ProtocolFeature::NativeEth => false, // V2 uses WETH
            ProtocolFeature::DynamicFees => false,
            ProtocolFeature::LimitOrders => false,
        }
    }
}