use alloy_primitives::{Address, B256};

/// Describes the exact AMM route (pool/protocol) to perform a buy swap.
///
/// Each variant carries the parameters needed to construct a router or
/// direct-pool swap transaction for that protocol.
#[derive(Debug, Clone, Copy)]
pub enum AmmSwapRoute {
    /// Uniswap V2-style pool (router-based swap)
    UniswapV2 { pool: Address },
    /// SushiSwap V2-style pool (router-based swap)
    SushiswapV2 { pool: Address },
    /// Generic Uniswap V2-style pool with an explicit router address.
    V2Router { pool: Address, router: Address },
    /// Uniswap V3 pool with fee tier (router-based exactInputSingle)
    UniswapV3 { pool: Address, fee_tier: u32 },
    /// Generic Uniswap V3-style pool with an explicit router address.
    V3Router {
        pool: Address,
        router: Address,
        fee_tier: u32,
    },
    /// Uniswap V4 via PoolManager (not yet supported by builders)
    /// Included for completeness so higher layers can select v4 and handle gracefully.
    UniswapV4 {
        pool_manager: Address,
        pool_id: B256,
    },
    /// Balancer V2 SingleSwap (not implemented yet)
    BalancerV2 {
        pool_id: B256,
        token_in: Address,
        token_out: Address,
    },
    /// Curve V1 pool (not implemented yet)
    CurveV1 {
        pool: Address,
        i: u8,
        j: u8,
        use_underlying: bool,
    },
    /// Fraxswap V2-style pool (router-based swap) (not implemented yet)
    FraxswapV2 { pool: Address },
}
