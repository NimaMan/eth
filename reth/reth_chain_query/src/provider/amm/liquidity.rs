use alloy_primitives::{Address, U256};
use eyre::Result;

use crate::provider::RethQueryProvider;
use crate::tx_builders::amm_swap_route::AmmSwapRoute;
use crate::common_addresses::DENOM_ADDRESSES;

/// Normalized liquidity info across AMM routes.
#[derive(Debug, Clone)]
pub struct PoolLiquidityInfo {
    pub protocol: &'static str,
    pub pool: Address,
    pub token0: Option<Address>,
    pub token1: Option<Address>,
    pub token0_symbol: Option<String>,
    pub token1_symbol: Option<String>,
    pub reserve0: Option<U256>,
    pub reserve1: Option<U256>,
    pub v3_liquidity: Option<U256>,
    pub tick: Option<i32>,
    pub block_number: u64,
}

impl RethQueryProvider {
    /// Get liquidity info for a specific AMM route at a block.
    ///
    /// - UniswapV2/SushiswapV2: returns reserves (reserve0, reserve1)
    /// - UniswapV3: returns (liquidity, tick)
    pub async fn get_route_liquidity(
        &self,
        route: &AmmSwapRoute,
        block: Option<u64>,
    ) -> Result<PoolLiquidityInfo> {
        let block_number = block.unwrap_or(self.get_latest_block()?);
        match *route {
            AmmSwapRoute::UniswapV2 { pool } => {
                // token0/token1 via token0()/token1() view
                let (t0, t1) = self.uni_v2_get_tokens(pool, Some(block_number)).await.unwrap_or((Address::ZERO, Address::ZERO));
                let s0 = DENOM_ADDRESSES.get(&t0).map(|s| (*s).to_string());
                let s1 = DENOM_ADDRESSES.get(&t1).map(|s| (*s).to_string());
                let (r0, r1, _ts) = self.uni_v2_get_reserves(pool, Some(block_number)).await?;
                Ok(PoolLiquidityInfo {
                    protocol: "UniswapV2",
                    pool,
                    token0: Some(t0),
                    token1: Some(t1),
                    token0_symbol: s0,
                    token1_symbol: s1,
                    reserve0: Some(r0),
                    reserve1: Some(r1),
                    v3_liquidity: None,
                    tick: None,
                    block_number,
                })
            }
            AmmSwapRoute::SushiswapV2 { pool } => {
                let (t0, t1) = self.uni_v2_get_tokens(pool, Some(block_number)).await.unwrap_or((Address::ZERO, Address::ZERO));
                let s0 = DENOM_ADDRESSES.get(&t0).map(|s| (*s).to_string());
                let s1 = DENOM_ADDRESSES.get(&t1).map(|s| (*s).to_string());
                let (r0, r1, _ts) = self.uni_v2_get_reserves(pool, Some(block_number)).await?;
                Ok(PoolLiquidityInfo {
                    protocol: "SushiswapV2",
                    pool,
                    token0: Some(t0),
                    token1: Some(t1),
                    token0_symbol: s0,
                    token1_symbol: s1,
                    reserve0: Some(r0),
                    reserve1: Some(r1),
                    v3_liquidity: None,
                    tick: None,
                    block_number,
                })
            }
            AmmSwapRoute::UniswapV3 { pool, .. } => {
                // token addresses via token0()/token1() as well
                let (t0, t1) = self.uni_v2_get_tokens(pool, Some(block_number)).await.unwrap_or((Address::ZERO, Address::ZERO));
                let s0 = DENOM_ADDRESSES.get(&t0).map(|s| (*s).to_string());
                let s1 = DENOM_ADDRESSES.get(&t1).map(|s| (*s).to_string());
                let (_sqrt, tick, liq, _ts) = self.uni_v3_get_slot0_and_liquidity(pool, Some(block_number)).await?;
                Ok(PoolLiquidityInfo {
                    protocol: "UniswapV3",
                    pool,
                    token0: Some(t0),
                    token1: Some(t1),
                    token0_symbol: s0,
                    token1_symbol: s1,
                    reserve0: None,
                    reserve1: None,
                    v3_liquidity: Some(liq),
                    tick: Some(tick),
                    block_number,
                })
            }
            AmmSwapRoute::UniswapV4 { pool_manager, pool_id } => {
                // V4: fetch slot0 + liquidity via PoolManager
                let (_sqrt, tick, liq, _ts) = self.uni_v4_get_slot0_and_liquidity(pool_manager, pool_id, Some(block_number)).await?;
                Ok(PoolLiquidityInfo {
                    protocol: "UniswapV4",
                    pool: pool_manager,
                    token0: None,
                    token1: None,
                    token0_symbol: None,
                    token1_symbol: None,
                    reserve0: None,
                    reserve1: None,
                    v3_liquidity: Some(liq),
                    tick: Some(tick),
                    block_number,
                })
            }
            // Curve/Balancer/Fraxswap: to be added in a later pass.
            _ => Err(eyre::eyre!("Route not supported for liquidity query")),
        }
    }
}
