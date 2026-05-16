use crate::pools::base::BasePoolConfig;
use crate::pools::uniswap::v2::UniswapV2Pool;

pub const PANCAKESWAP_V2_PROTOCOL: &str = "PANCAKESWAP-V2";
pub const PANCAKESWAP_V2_FACTORY: &str = "0x1097053fd2ea711dad45caccc45eff7548fcb362";

pub type PancakeSwapV2Pool = UniswapV2Pool;

pub fn new_pancakeswap_v2_pool(
    pool_address: impl Into<String>,
    token_address: impl Into<String>,
    denom_address: impl Into<String>,
    config: BasePoolConfig,
    known_routers: impl IntoIterator<Item = impl AsRef<str>>,
) -> PancakeSwapV2Pool {
    UniswapV2Pool::new_with_protocol(
        pool_address,
        token_address,
        denom_address,
        PANCAKESWAP_V2_PROTOCOL,
        config,
        known_routers,
    )
}
