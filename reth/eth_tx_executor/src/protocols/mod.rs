//! Protocol abstractions for DEX integrations
//! 
//! Provides trait-based abstractions that allow the system to work with
//! multiple DEX protocols (Uniswap V2, V3, V4, etc.) through a common interface.

pub mod traits;

// Protocol implementations
pub mod uniswap_v2;
// TODO: pub mod uniswap_v3;
// TODO: pub mod uniswap_v4;

// Re-export core traits
pub use traits::{
    DexPool, DexProtocol, LiquidityPool, PoolInfo, ProtocolFactory,
    SwapQuote, SwapRoute,
};

use crate::common::{Result, TokenAddress};
use ethers::prelude::*;
use std::sync::Arc;

/// Registry of all available protocols
pub struct ProtocolRegistry {
    protocols: Vec<Box<dyn DexProtocol>>,
}

impl ProtocolRegistry {
    /// Create a new protocol registry
    pub fn new(provider: Arc<Provider<Http>>) -> Self {
        let mut protocols: Vec<Box<dyn DexProtocol>> = Vec::new();
        
        // Register Uniswap V2
        protocols.push(Box::new(uniswap_v2::UniswapV2Protocol::new(provider.clone())));
        
        // TODO: Register other protocols as they're implemented
        // protocols.push(Box::new(uniswap_v3::UniswapV3Protocol::new(provider.clone())));
        
        Self { protocols }
    }
    
    /// Find the best pool across all protocols for a token pair
    pub async fn find_best_pool(
        &self,
        token_a: TokenAddress,
        token_b: TokenAddress,
        amount_in: U256,
    ) -> Result<(Box<dyn DexPool>, SwapQuote)> {
        let mut best_quote: Option<(Box<dyn DexPool>, SwapQuote)> = None;
        let mut best_output = U256::zero();
        
        for protocol in &self.protocols {
            // Try to find pool in this protocol
            match protocol.find_pool(token_a, token_b).await {
                Ok(Some(pool)) => {
                    // Get quote from this pool
                    match pool.get_swap_quote(token_a, token_b, amount_in).await {
                        Ok(quote) => {
                            if quote.amount_out > best_output {
                                best_output = quote.amount_out;
                                best_quote = Some((pool, quote));
                            }
                        }
                        Err(e) => {
                            tracing::debug!(
                                "Failed to get quote from {} pool: {}",
                                protocol.name(),
                                e
                            );
                        }
                    }
                }
                Ok(None) => {
                    tracing::debug!(
                        "{} has no pool for {:?}/{:?}",
                        protocol.name(),
                        token_a,
                        token_b
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Error finding pool in {}: {}",
                        protocol.name(),
                        e
                    );
                }
            }
        }
        
        best_quote.ok_or_else(|| {
            crate::common::errors::KartalError::Pool(
                crate::common::errors::PoolError::PoolNotFound {
                    address: Address::zero(), // No specific address
                }
            )
        })
    }
    
    /// Get all available pools for a token pair
    pub async fn get_all_pools(
        &self,
        token_a: TokenAddress,
        token_b: TokenAddress,
    ) -> Vec<(String, Box<dyn DexPool>)> {
        let mut pools = Vec::new();
        
        for protocol in &self.protocols {
            if let Ok(Some(pool)) = protocol.find_pool(token_a, token_b).await {
                pools.push((protocol.name().to_string(), pool));
            }
        }
        
        pools
    }
    
    /// Get a specific protocol by name
    pub fn get_protocol(&self, name: &str) -> Option<&dyn DexProtocol> {
        self.protocols
            .iter()
            .find(|p| p.name().eq_ignore_ascii_case(name))
            .map(|p| p.as_ref())
    }
}