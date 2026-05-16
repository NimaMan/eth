use crate::pools::base::BasePoolConfig;
use crate::pools::uniswap::v3::UniswapV3Pool;

pub const PANCAKESWAP_V3_PROTOCOL: &str = "PANCAKESWAP-V3";

pub type PancakeSwapV3Pool = UniswapV3Pool;

pub fn new_pancakeswap_v3_pool(
    pool_address: impl Into<String>,
    token_address: impl Into<String>,
    denom_address: impl Into<String>,
    token0: impl Into<String>,
    token1: impl Into<String>,
    fee_tier: u32,
    tick_spacing: i32,
    config: BasePoolConfig,
) -> PancakeSwapV3Pool {
    UniswapV3Pool::new_with_protocol(
        pool_address,
        token_address,
        denom_address,
        token0,
        token1,
        fee_tier,
        tick_spacing,
        PANCAKESWAP_V3_PROTOCOL,
        config,
    )
}
