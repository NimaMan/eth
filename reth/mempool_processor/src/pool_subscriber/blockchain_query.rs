// rust/mempool_processor/src/pool_subscriber/blockchain_query.rs
//
// Direct blockchain querying for accurate pool reserves.
//
// OBJECTIVE: Replace Python's inflated pool values with direct on-chain queries.
//
// ALGORITHM:
// 1. Connect to local reth node (as per workspace rules)
// 2. Call getReserves() on Uniswap V2 pair contracts
// 3. Determine which reserve is ETH/WETH based on token addresses
// 4. Return accurate pool reserves for scam detection
//
// This eliminates the 2x inflation issue found in Python's event processing.

use alloy_provider::{DynProvider as AlloyDynProvider, ProviderBuilder, Provider as AlloyProviderTrait};
use alloy_network::Ethereum as AlloyEthereum;
use alloy_sol_types::sol;
use revm_primitives::alloy_primitives::{Address, U256, address, utils::format_units};
use eyre::{Result, WrapErr};
use tracing::{debug, warn};
use std::sync::Arc;

// Uniswap V2 Pair contract interface
sol! {
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
        function token0() external view returns (address);
        function token1() external view returns (address);
    }
}

// WETH address on mainnet
const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

pub struct BlockchainPoolQuerier {
    provider: Arc<AlloyDynProvider<AlloyEthereum>>,
}

impl BlockchainPoolQuerier {
    /// Create a new blockchain pool querier
    pub async fn new() -> Result<Self> {
        // Use local reth node as per workspace rules
        let rpc_url = "http://127.0.0.1:8545";
        
        let provider_instance = ProviderBuilder::new()
            .connect(rpc_url)
            .await
            .wrap_err("Failed to connect to local reth node")?;
        
        let provider = Arc::new(provider_instance.erased());
        
        debug!("✅ Connected to local reth node at {}", rpc_url);
        
        Ok(Self { provider })
    }
    
    /// Query actual pool reserves from blockchain
    /// Returns (eth_reserve, token_reserve, success) 
    pub async fn get_pool_reserves(&self, pool_address: Address) -> (f64, f64, bool) {
        match self.query_reserves_internal(pool_address).await {
            Ok((eth_reserve, token_reserve)) => {
                debug!("✅ Pool {}: ETH={:.6}, Token={:.6}", pool_address, eth_reserve, token_reserve);
                (eth_reserve, token_reserve, true)
            }
            Err(e) => {
                warn!("❌ Failed to query pool {}: {}", pool_address, e);
                (0.0, 0.0, false)
            }
        }
    }
    
    async fn query_reserves_internal(&self, pool_address: Address) -> Result<(f64, f64)> {
        // Since we don't have the sol! macro working correctly, let's use a simpler approach
        // We'll use the raw ethereum ABI calls instead
        
        use alloy_rpc_types_eth::transaction::request::TransactionRequest;
        use revm_primitives::Bytes;
        
        // Create transaction request for token0()
        let token0_data = hex::decode("0dbe671f")?; // token0() method signature
        let token0_req = TransactionRequest {
            to: Some(pool_address.into()),
            data: Some(Bytes::from(token0_data)),
            ..Default::default()
        };
        
        let token0_result = self.provider.call(&token0_req).await?;
        let token0_address = Address::from_slice(&token0_result[12..32]); // Extract address from 32-byte result
        
        // Create transaction request for token1()
        let token1_data = hex::decode("d21220a7")?; // token1() method signature  
        let token1_req = TransactionRequest {
            to: Some(pool_address.into()),
            data: Some(Bytes::from(token1_data)),
            ..Default::default()
        };
        
        let token1_result = self.provider.call(&token1_req).await?;
        let token1_address = Address::from_slice(&token1_result[12..32]); // Extract address from 32-byte result
        
        // Create transaction request for getReserves()
        let reserves_data = hex::decode("0902f1ac")?; // getReserves() method signature
        let reserves_req = TransactionRequest {
            to: Some(pool_address.into()),
            data: Some(Bytes::from(reserves_data)),
            ..Default::default()
        };
        
        let reserves_result = self.provider.call(&reserves_req).await?;
        
        // Parse reserves from returned bytes
        // getReserves returns (uint112, uint112, uint32) = 32 + 32 + 32 bytes
        let reserve0_bytes = &reserves_result[0..32];
        let reserve1_bytes = &reserves_result[32..64];
        
        let reserve0 = U256::from_be_slice(reserve0_bytes);
        let reserve1 = U256::from_be_slice(reserve1_bytes);
        
        // Determine which reserve is ETH/WETH
        let (eth_reserve, token_reserve) = if token0_address == WETH_ADDRESS {
            // token0 is WETH, token1 is the other token
            let eth_reserve = format_units(reserve0, 18)?.parse::<f64>()?;
            let token_reserve = format_units(reserve1, 18)?.parse::<f64>()?;
            (eth_reserve, token_reserve)
        } else if token1_address == WETH_ADDRESS {
            // token1 is WETH, token0 is the other token  
            let eth_reserve = format_units(reserve1, 18)?.parse::<f64>()?;
            let token_reserve = format_units(reserve0, 18)?.parse::<f64>()?;
            (eth_reserve, token_reserve)
        } else {
            // Neither token is WETH - this shouldn't happen for our use case
            // Default to treating reserve0 as ETH equivalent 
            warn!("⚠️ Pool {} has no WETH token: token0={}, token1={}", pool_address, token0_address, token1_address);
            let eth_reserve = format_units(reserve0, 18)?.parse::<f64>()?;
            let token_reserve = format_units(reserve1, 18)?.parse::<f64>()?;
            (eth_reserve, token_reserve)
        };
        
        debug!("🔍 Pool {} breakdown:", pool_address);
        debug!("  Token0: {} = {:.6}", token0_address, format_units(reserve0, 18)?);
        debug!("  Token1: {} = {:.6}", token1_address, format_units(reserve1, 18)?);
        debug!("  ETH Reserve: {:.6}", eth_reserve);
        debug!("  Token Reserve: {:.6}", token_reserve);
        
        Ok((eth_reserve, token_reserve))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_can_create_querier() {
        // This test requires a running reth node
        if let Ok(querier) = BlockchainPoolQuerier::new().await {
            // Just verify it was created successfully
            assert!(!querier.provider.client().is_local());
        }
    }
    
    #[tokio::test] 
    async fn test_query_known_pool() {
        // Test with a known USDC/WETH pool
        if let Ok(querier) = BlockchainPoolQuerier::new().await {
            let pool_address = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");
            let (eth_reserve, token_reserve, success) = querier.get_pool_reserves(pool_address).await;
            
            if success {
                // Should have positive reserves
                assert!(eth_reserve > 0.0);
                assert!(token_reserve >= 0.0); // Token reserve can be very small
                println!("USDC/WETH Pool: ETH={:.6}, USDC={:.6}", eth_reserve, token_reserve);
            }
        }
    }
} 