use alloy_primitives::{Address, U256, B256};
use eyre::Result;

use crate::provider::RethQueryProvider;
use crate::tx_builders::amm_swap_route::AmmSwapRoute;
use crate::common_addresses::DENOM_ADDRESSES;

/// Normalized liquidity info across AMM routes.
#[derive(Debug, Clone)]
pub struct PoolLiquidityInfo {
    pub protocol: &'static str,
    pub pool: Address,
    pub pool_id: Option<B256>,
    pub token0: Option<Address>,
    pub token1: Option<Address>,
    pub token0_symbol: Option<String>,
    pub token1_symbol: Option<String>,
    pub token0_decimals: Option<u8>,
    pub token1_decimals: Option<u8>,
    pub reserve0: Option<U256>,
    pub reserve1: Option<U256>,
    pub v3_liquidity: Option<U256>,
    pub tick: Option<i32>,
    pub sqrt_price_x96: Option<U256>,
    pub price_1e18: Option<U256>, /// token1 per 1 token0, scaled by 1e18
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
                let (d0, d1) = match tokio::try_join!(
                    self.get_token_decimals(t0, Some(block_number)),
                    self.get_token_decimals(t1, Some(block_number)),
                ) {
                    Ok((a, b)) => (a, b),
                    Err(_) => (18u8, 18u8),
                };
                let price = compute_price_from_reserves_1e18(r0, r1, d0, d1);
                Ok(PoolLiquidityInfo {
                    protocol: "Uniswap-V2",
                    pool,
                    pool_id: None,
                    token0: Some(t0),
                    token1: Some(t1),
                    token0_symbol: s0,
                    token1_symbol: s1,
                    token0_decimals: Some(d0),
                    token1_decimals: Some(d1),
                    reserve0: Some(r0),
                    reserve1: Some(r1),
                    v3_liquidity: None,
                    tick: None,
                    sqrt_price_x96: None,
                    price_1e18: Some(price),
                    block_number,
                })
            }
            AmmSwapRoute::SushiswapV2 { pool } => {
                let (t0, t1) = self.uni_v2_get_tokens(pool, Some(block_number)).await.unwrap_or((Address::ZERO, Address::ZERO));
                let s0 = DENOM_ADDRESSES.get(&t0).map(|s| (*s).to_string());
                let s1 = DENOM_ADDRESSES.get(&t1).map(|s| (*s).to_string());
                let (r0, r1, _ts) = self.uni_v2_get_reserves(pool, Some(block_number)).await?;
                let (d0, d1) = match tokio::try_join!(
                    self.get_token_decimals(t0, Some(block_number)),
                    self.get_token_decimals(t1, Some(block_number)),
                ) {
                    Ok((a, b)) => (a, b),
                    Err(_) => (18u8, 18u8),
                };
                let price = compute_price_from_reserves_1e18(r0, r1, d0, d1);
                Ok(PoolLiquidityInfo {
                    protocol: "SushiswapV2",
                    pool,
                    pool_id: None,
                    token0: Some(t0),
                    token1: Some(t1),
                    token0_symbol: s0,
                    token1_symbol: s1,
                    token0_decimals: Some(d0),
                    token1_decimals: Some(d1),
                    reserve0: Some(r0),
                    reserve1: Some(r1),
                    v3_liquidity: None,
                    tick: None,
                    sqrt_price_x96: None,
                    price_1e18: Some(price),
                    block_number,
                })
            }
            AmmSwapRoute::UniswapV3 { pool, .. } => {
                // token addresses via token0()/token1() as well
                let (t0, t1) = self.uni_v2_get_tokens(pool, Some(block_number)).await.unwrap_or((Address::ZERO, Address::ZERO));
                let s0 = DENOM_ADDRESSES.get(&t0).map(|s| (*s).to_string());
                let s1 = DENOM_ADDRESSES.get(&t1).map(|s| (*s).to_string());
                let (sqrt, tick, liq, _ts) = self.uni_v3_get_slot0_and_liquidity(pool, Some(block_number)).await?;
                let (d0, d1) = match tokio::try_join!(
                    self.get_token_decimals(t0, Some(block_number)),
                    self.get_token_decimals(t1, Some(block_number)),
                ) {
                    Ok((a, b)) => (a, b),
                    Err(_) => (18u8, 18u8),
                };
                let price = compute_price_from_sqrt_price_1e18(sqrt, d0, d1);
                Ok(PoolLiquidityInfo {
                    protocol: "Uniswap-V3",
                    pool,
                    pool_id: None,
                    token0: Some(t0),
                    token1: Some(t1),
                    token0_symbol: s0,
                    token1_symbol: s1,
                    token0_decimals: Some(d0),
                    token1_decimals: Some(d1),
                    reserve0: None,
                    reserve1: None,
                    v3_liquidity: Some(liq),
                    tick: Some(tick),
                    sqrt_price_x96: Some(sqrt),
                    price_1e18: Some(price),
                    block_number,
                })
            }
            AmmSwapRoute::UniswapV4 { pool_manager, pool_id } => {
                // V4: fetch slot0 + liquidity via PoolManager
                let (sqrt, tick, liq, _ts) = self.uni_v4_get_slot0_and_liquidity(pool_manager, pool_id, Some(block_number)).await?;
                Ok(PoolLiquidityInfo {
                    protocol: "Uniswap-V4",
                    pool: pool_manager,
                    pool_id: Some(pool_id),
                    token0: None,
                    token1: None,
                    token0_symbol: None,
                    token1_symbol: None,
                    token0_decimals: None,
                    token1_decimals: None,
                    reserve0: None,
                    reserve1: None,
                    v3_liquidity: Some(liq),
                    tick: Some(tick),
                    sqrt_price_x96: Some(sqrt),
                    price_1e18: None,
                    block_number,
                })
            }
            AmmSwapRoute::BalancerV2 { pool_id, token_in, token_out } => {
                // Balancer: fetch tokens and balances from Vault (no pricing here)
                let (pool_addr, _spec) = self.balancer_v2_get_pool_info(pool_id, Some(block_number)).await?;
                let (tokens, balances, _last_block) = self.balancer_v2_get_pool_tokens_and_balances(pool_id, Some(block_number)).await?;
                // Map provided tokens to their balances
                let mut bal_map: std::collections::HashMap<Address, U256> = std::collections::HashMap::new();
                for (i, t) in tokens.iter().enumerate() {
                    if let Some(b) = balances.get(i) { bal_map.insert(*t, *b); }
                }
                // Ensure deterministic orientation: sort by address
                let (t0, t1) = if token_in < token_out { (token_in, token_out) } else { (token_out, token_in) };
                let r0 = *bal_map.get(&t0).unwrap_or(&U256::ZERO);
                let r1 = *bal_map.get(&t1).unwrap_or(&U256::ZERO);
                let s0 = DENOM_ADDRESSES.get(&t0).map(|s| (*s).to_string());
                let s1 = DENOM_ADDRESSES.get(&t1).map(|s| (*s).to_string());
                // Try decimals for both tokens
                let (d0, d1) = match tokio::try_join!(
                    self.get_token_decimals(t0, Some(block_number)),
                    self.get_token_decimals(t1, Some(block_number)),
                ) { Ok((a,b)) => (a,b), Err(_) => (18,18) };
                Ok(PoolLiquidityInfo {
                    protocol: "Balancer-V2",
                    pool: pool_addr,
                    pool_id: Some(pool_id),
                    token0: Some(t0),
                    token1: Some(t1),
                    token0_symbol: s0,
                    token1_symbol: s1,
                    token0_decimals: Some(d0),
                    token1_decimals: Some(d1),
                    reserve0: Some(r0),
                    reserve1: Some(r1),
                    v3_liquidity: None,
                    tick: None,
                    sqrt_price_x96: None,
                    price_1e18: None,
                    block_number,
                })
            }
            AmmSwapRoute::CurveV1 { pool, i, j, use_underlying } => {
                // Curve: fetch coin addresses and balances by index
                let a = self.curve_v1_get_coin(pool, i, use_underlying, Some(block_number)).await.unwrap_or(Address::ZERO);
                let b = self.curve_v1_get_coin(pool, j, use_underlying, Some(block_number)).await.unwrap_or(Address::ZERO);
                let ra = self.curve_v1_get_balance(pool, i, Some(block_number)).await.unwrap_or(U256::ZERO);
                let rb = self.curve_v1_get_balance(pool, j, Some(block_number)).await.unwrap_or(U256::ZERO);
                // Sort for consistent orientation
                let (t0, t1, r0, r1) = if a <= b { (a, b, ra, rb) } else { (b, a, rb, ra) };
                let s0 = DENOM_ADDRESSES.get(&t0).map(|s| (*s).to_string());
                let s1 = DENOM_ADDRESSES.get(&t1).map(|s| (*s).to_string());
                let (d0, d1) = match tokio::try_join!(
                    self.get_token_decimals(t0, Some(block_number)),
                    self.get_token_decimals(t1, Some(block_number)),
                ) { Ok((a,b)) => (a,b), Err(_) => (18,18) };
                let price = compute_price_from_reserves_1e18(r0, r1, d0, d1);
                Ok(PoolLiquidityInfo {
                    protocol: "Curve-V1",
                    pool,
                    pool_id: None,
                    token0: Some(t0),
                    token1: Some(t1),
                    token0_symbol: s0,
                    token1_symbol: s1,
                    token0_decimals: Some(d0),
                    token1_decimals: Some(d1),
                    reserve0: Some(r0),
                    reserve1: Some(r1),
                    v3_liquidity: None,
                    tick: None,
                    sqrt_price_x96: None,
                    price_1e18: Some(price),
                    block_number,
                })
            }
            // Curve/Balancer/Fraxswap: to be added in a later pass.
            _ => Err(eyre::eyre!("Route not supported for liquidity query")),
        }
    }
}

fn compute_price_from_reserves_1e18(r0: U256, r1: U256, d0: u8, d1: u8) -> U256 {
    if r0.is_zero() { return U256::ZERO; }
    // price = (r1 / 10^d1) / (r0 / 10^d0)
    // scaled by 1e18
    let scale_18 = pow10_u256(18);
    if d0 >= d1 {
        let adj = pow10_u256((d0 - d1) as u32);
        r1.saturating_mul(adj).saturating_mul(scale_18) / r0
    } else {
        let adj = pow10_u256((d1 - d0) as u32);
        if r0.is_zero() || adj.is_zero() { U256::ZERO } else { r1.saturating_mul(scale_18) / (r0.saturating_mul(adj)) }
    }
}

fn compute_price_from_sqrt_price_1e18(sqrt_price_x96: U256, d0: u8, d1: u8) -> U256 {
    if sqrt_price_x96.is_zero() { return U256::ZERO; }
    // price = (sqrtP^2 / 2^192)
    let sqrt_sq = sqrt_price_x96.saturating_mul(sqrt_price_x96);
    let two_192 = U256::from(1u128) << 192;
    let ratio: U256 = sqrt_sq / two_192;
    let scale_18 = pow10_u256(18);
    if d0 >= d1 {
        let adj = pow10_u256((d0 - d1) as u32);
        ratio.saturating_mul(adj).saturating_mul(scale_18)
    } else {
        let adj = pow10_u256((d1 - d0) as u32);
        if adj.is_zero() { U256::ZERO } else { ratio.saturating_mul(scale_18) / adj }
    }
}

fn pow10_u256(exp: u32) -> U256 {
    let mut out = U256::from(1);
    for _ in 0..exp { out = out.saturating_mul(U256::from(10)); }
    out
}

// Intentionally no WETH-equivalent helpers here; keep pricing minimal in RCQ.
