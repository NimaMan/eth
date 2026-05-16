use crate::pools::base::BasePoolConfig;
use crate::pools::uniswap::v3::UniswapV3Pool;

pub const SUSHISWAP_V3_PROTOCOL: &str = "SUSHISWAP-V3";

pub type SushiSwapV3Pool = UniswapV3Pool;

pub fn new_sushiswap_v3_pool(
    pool_address: impl Into<String>,
    token_address: impl Into<String>,
    denom_address: impl Into<String>,
    token0: impl Into<String>,
    token1: impl Into<String>,
    fee_tier: u32,
    tick_spacing: i32,
    config: BasePoolConfig,
) -> SushiSwapV3Pool {
    UniswapV3Pool::new_with_protocol(
        pool_address,
        token_address,
        denom_address,
        token0,
        token1,
        fee_tier,
        tick_spacing,
        SUSHISWAP_V3_PROTOCOL,
        config,
    )
}
