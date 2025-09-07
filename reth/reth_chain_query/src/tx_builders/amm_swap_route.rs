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
    /// Uniswap V3 pool with fee tier (router-based exactInputSingle)
    UniswapV3 { pool: Address, fee_tier: u32 },
    /// Balancer V2 SingleSwap (not implemented yet)
    BalancerV2 { pool_id: B256, token_in: Address, token_out: Address },
    /// Curve V1 pool (not implemented yet)
    CurveV1 { pool: Address, i: u8, j: u8, use_underlying: bool },
    /// Fraxswap V2-style pool (router-based swap) (not implemented yet)
    FraxswapV2 { pool: Address },
}

