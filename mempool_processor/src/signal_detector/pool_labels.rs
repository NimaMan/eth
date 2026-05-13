use crate::token_tracking::types::PoolType;

pub(crate) fn pool_type_label(pool_type: &PoolType) -> String {
    match pool_type {
        PoolType::UniswapV2 => "UNISWAP-V2",
        PoolType::UniswapV3 => "UNISWAP-V3",
        PoolType::UniswapV4 => "UNISWAP-V4",
        PoolType::SushiSwapV2 => "SUSHISWAP-V2",
        PoolType::SushiSwapV3 => "SUSHISWAP-V3",
        PoolType::PancakeSwapV2 => "PANCAKESWAP-V2",
        PoolType::PancakeSwapV3 => "PANCAKESWAP-V3",
        PoolType::ShibaSwapV2 => "SHIBASWAP-V2",
        PoolType::FraxswapV2 => "FRAXSWAP-V2",
        PoolType::Curve => "CURVE",
        PoolType::Balancer => "BALANCER",
        PoolType::Unknown => "UNKNOWN",
    }
    .to_string()
}
