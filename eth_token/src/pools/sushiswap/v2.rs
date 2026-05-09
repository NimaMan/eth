use crate::pools::base::BasePoolConfig;
use crate::pools::uniswap::v2::UniswapV2Pool;

pub const SUSHISWAP_V2_PROTOCOL: &str = "SUSHISWAP-V2";
pub const SUSHISWAP_V2_FACTORY: &str = "0xc0aee478e3658e2610c5f7a4a2e1777ce9e4f2ac";

pub type SushiSwapV2Pool = UniswapV2Pool;

pub fn new_sushiswap_v2_pool(
    pool_address: impl Into<String>,
    token_address: impl Into<String>,
    denom_address: impl Into<String>,
    config: BasePoolConfig,
    known_routers: impl IntoIterator<Item = impl AsRef<str>>,
) -> SushiSwapV2Pool {
    UniswapV2Pool::new_with_protocol(
        pool_address,
        token_address,
        denom_address,
        SUSHISWAP_V2_PROTOCOL,
        config,
        known_routers,
    )
}
