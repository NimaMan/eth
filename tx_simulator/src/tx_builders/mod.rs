pub mod amm_swap_route;
pub mod curve;
pub mod permit2;
pub mod uniswap_v2;
pub mod uniswap_v3;
pub mod uniswap_v4;

use self::amm_swap_route::AmmSwapRoute;
use crate::UnsignedTransaction;
use alloy_primitives::{Address, U256};

/// Build a buy (ETH -> Token) swap for the given AMM route.
pub fn build_buy_swap(
    route: &AmmSwapRoute,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => uniswap_v2::build_buy_swap_v2(
            uniswap_v2::Router::UniswapV2,
            buyer,
            token_out,
            amount_in_eth,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => uniswap_v2::build_buy_swap_v2(
            uniswap_v2::Router::SushiswapV2,
            buyer,
            token_out,
            amount_in_eth,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::V2Router { router, .. } => uniswap_v2::build_buy_swap_v2(
            uniswap_v2::Router::Custom(router),
            buyer,
            token_out,
            amount_in_eth,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => uniswap_v3::build_buy_swap_v3(
            buyer,
            token_out,
            amount_in_eth,
            fee_tier,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::V3Router {
            router, fee_tier, ..
        } => uniswap_v3::build_buy_swap_v3_with_router(
            router,
            buyer,
            token_out,
            amount_in_eth,
            fee_tier,
            deadline,
        ),
        AmmSwapRoute::CurveV1 {
            pool,
            i,
            j,
            use_underlying,
        } => curve::build_buy_swap_curve_v1(
            buyer,
            pool,
            i,
            j,
            amount_in_eth,
            use_underlying,
            U256::ZERO,
        ),
        _ => uniswap_v2::build_buy_swap_v2(
            uniswap_v2::Router::UniswapV2,
            buyer,
            token_out,
            amount_in_eth,
            slippage_bps,
            deadline,
        ),
    }
}

/// Build a buy swap with explicit amountOutMin.
pub fn build_buy_swap_with_min_out(
    route: &AmmSwapRoute,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => uniswap_v2::build_buy_swap_v2_with_min_out(
            uniswap_v2::Router::UniswapV2,
            buyer,
            token_out,
            amount_in_eth,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => uniswap_v2::build_buy_swap_v2_with_min_out(
            uniswap_v2::Router::SushiswapV2,
            buyer,
            token_out,
            amount_in_eth,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::V2Router { router, .. } => uniswap_v2::build_buy_swap_v2_with_min_out(
            uniswap_v2::Router::Custom(router),
            buyer,
            token_out,
            amount_in_eth,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => uniswap_v3::build_buy_swap_v3_with_min_out(
            buyer,
            token_out,
            amount_in_eth,
            fee_tier,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::V3Router {
            router, fee_tier, ..
        } => uniswap_v3::build_buy_swap_v3_with_min_out_router(
            router,
            buyer,
            token_out,
            amount_in_eth,
            fee_tier,
            amount_out_min,
            deadline,
        ),
        _ => uniswap_v2::build_buy_swap_v2_with_min_out(
            uniswap_v2::Router::UniswapV2,
            buyer,
            token_out,
            amount_in_eth,
            amount_out_min,
            deadline,
        ),
    }
}

/// Build an approve transaction for the given AMM route.
pub fn build_approve_for_route(
    route: &AmmSwapRoute,
    owner: Address,
    token: Address,
    amount: U256,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => {
            uniswap_v2::build_approve_v2(uniswap_v2::Router::UniswapV2, owner, token, amount)
        }
        AmmSwapRoute::SushiswapV2 { .. } => {
            uniswap_v2::build_approve_v2(uniswap_v2::Router::SushiswapV2, owner, token, amount)
        }
        AmmSwapRoute::V2Router { router, .. } => {
            uniswap_v2::build_approve_v2(uniswap_v2::Router::Custom(router), owner, token, amount)
        }
        AmmSwapRoute::UniswapV3 { .. } => uniswap_v3::build_approve_v3(owner, token, amount),
        AmmSwapRoute::V3Router { router, .. } => {
            uniswap_v3::build_approve_v3_for_router(router, owner, token, amount)
        }
        _ => uniswap_v2::build_approve_v2(uniswap_v2::Router::UniswapV2, owner, token, amount),
    }
}

/// Build a sell (Token -> ETH) swap for the given AMM route.
pub fn build_sell_swap(
    route: &AmmSwapRoute,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => uniswap_v2::build_sell_swap_v2(
            uniswap_v2::Router::UniswapV2,
            seller,
            token_in,
            amount_in_tokens,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => uniswap_v2::build_sell_swap_v2(
            uniswap_v2::Router::SushiswapV2,
            seller,
            token_in,
            amount_in_tokens,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::V2Router { router, .. } => uniswap_v2::build_sell_swap_v2(
            uniswap_v2::Router::Custom(router),
            seller,
            token_in,
            amount_in_tokens,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => uniswap_v3::build_sell_swap_v3(
            seller,
            token_in,
            amount_in_tokens,
            fee_tier,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::V3Router {
            router, fee_tier, ..
        } => uniswap_v3::build_sell_swap_v3_with_router(
            router,
            seller,
            token_in,
            amount_in_tokens,
            fee_tier,
            deadline,
        ),
        _ => uniswap_v2::build_sell_swap_v2(
            uniswap_v2::Router::UniswapV2,
            seller,
            token_in,
            amount_in_tokens,
            slippage_bps,
            deadline,
        ),
    }
}

/// Build a denomination-token -> target-token swap for pool viability checks.
///
/// V2-style routers use the fee-on-transfer supporting token-to-token selector so
/// taxed output tokens do not trip the plain router path.
pub fn build_denom_to_token_swap(
    route: &AmmSwapRoute,
    trader: Address,
    denom_in: Address,
    token_out: Address,
    amount_in: U256,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_fee_tolerant_token_to_token_swap(route, trader, denom_in, token_out, amount_in, deadline)
}

/// Build a target-token -> denomination-token swap for pool viability checks.
///
/// V2-style routers use the fee-on-transfer supporting token-to-token selector so
/// taxed input tokens can be sold without the router's plain transfer check.
pub fn build_token_to_denom_swap(
    route: &AmmSwapRoute,
    trader: Address,
    token_in: Address,
    denom_out: Address,
    amount_in: U256,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_fee_tolerant_token_to_token_swap(route, trader, token_in, denom_out, amount_in, deadline)
}

fn build_fee_tolerant_token_to_token_swap(
    route: &AmmSwapRoute,
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => uniswap_v2::build_token_to_token_swap_supporting_fee_v2(
            uniswap_v2::Router::UniswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => {
            uniswap_v2::build_token_to_token_swap_supporting_fee_v2(
                uniswap_v2::Router::SushiswapV2,
                trader,
                token_in,
                token_out,
                amount_in,
                deadline,
            )
        }
        AmmSwapRoute::V2Router { router, .. } => {
            uniswap_v2::build_token_to_token_swap_supporting_fee_v2(
                uniswap_v2::Router::Custom(router),
                trader,
                token_in,
                token_out,
                amount_in,
                deadline,
            )
        }
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => uniswap_v3::build_token_to_token_swap_v3(
            trader, token_in, token_out, amount_in, fee_tier, 0, deadline,
        ),
        AmmSwapRoute::V3Router {
            router, fee_tier, ..
        } => uniswap_v3::build_token_to_token_swap_v3_with_router(
            router, trader, token_in, token_out, amount_in, fee_tier, deadline,
        ),
        _ => build_token_to_token_swap(route, trader, token_in, token_out, amount_in, 0, deadline),
    }
}

/// Build a sell swap with explicit amountOutMin.
pub fn build_sell_swap_with_min_out(
    route: &AmmSwapRoute,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => uniswap_v2::build_sell_swap_v2_with_min_out(
            uniswap_v2::Router::UniswapV2,
            seller,
            token_in,
            amount_in_tokens,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => uniswap_v2::build_sell_swap_v2_with_min_out(
            uniswap_v2::Router::SushiswapV2,
            seller,
            token_in,
            amount_in_tokens,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::V2Router { router, .. } => uniswap_v2::build_sell_swap_v2_with_min_out(
            uniswap_v2::Router::Custom(router),
            seller,
            token_in,
            amount_in_tokens,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => uniswap_v3::build_sell_swap_v3_with_min_out(
            seller,
            token_in,
            amount_in_tokens,
            fee_tier,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::V3Router {
            router, fee_tier, ..
        } => uniswap_v3::build_sell_swap_v3_with_min_out_router(
            router,
            seller,
            token_in,
            amount_in_tokens,
            fee_tier,
            amount_out_min,
            deadline,
        ),
        _ => uniswap_v2::build_sell_swap_v2_with_min_out(
            uniswap_v2::Router::UniswapV2,
            seller,
            token_in,
            amount_in_tokens,
            amount_out_min,
            deadline,
        ),
    }
}

/// Build a token -> token swap for the given AMM route.
pub fn build_token_to_token_swap(
    route: &AmmSwapRoute,
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => uniswap_v2::build_token_to_token_swap_v2(
            uniswap_v2::Router::UniswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => uniswap_v2::build_token_to_token_swap_v2(
            uniswap_v2::Router::SushiswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::V2Router { router, .. } => uniswap_v2::build_token_to_token_swap_v2(
            uniswap_v2::Router::Custom(router),
            trader,
            token_in,
            token_out,
            amount_in,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => uniswap_v3::build_token_to_token_swap_v3(
            trader,
            token_in,
            token_out,
            amount_in,
            fee_tier,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::V3Router {
            router, fee_tier, ..
        } => uniswap_v3::build_token_to_token_swap_v3_with_router(
            router, trader, token_in, token_out, amount_in, fee_tier, deadline,
        ),
        _ => uniswap_v2::build_token_to_token_swap_v2(
            uniswap_v2::Router::UniswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            slippage_bps,
            deadline,
        ),
    }
}

/// Build a token -> token swap with explicit amountOutMin.
pub fn build_token_to_token_swap_with_min_out(
    route: &AmmSwapRoute,
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => uniswap_v2::build_token_to_token_swap_v2_with_min_out(
            uniswap_v2::Router::UniswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => uniswap_v2::build_token_to_token_swap_v2_with_min_out(
            uniswap_v2::Router::SushiswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::V2Router { router, .. } => {
            uniswap_v2::build_token_to_token_swap_v2_with_min_out(
                uniswap_v2::Router::Custom(router),
                trader,
                token_in,
                token_out,
                amount_in,
                amount_out_min,
                deadline,
            )
        }
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => {
            uniswap_v3::build_token_to_token_swap_v3_with_min_out(
                trader,
                token_in,
                token_out,
                amount_in,
                fee_tier,
                amount_out_min,
                deadline,
            )
        }
        AmmSwapRoute::V3Router {
            router, fee_tier, ..
        } => uniswap_v3::build_token_to_token_swap_v3_with_min_out_router(
            router,
            trader,
            token_in,
            token_out,
            amount_in,
            fee_tier,
            amount_out_min,
            deadline,
        ),
        _ => uniswap_v2::build_token_to_token_swap_v2_with_min_out(
            uniswap_v2::Router::UniswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            amount_out_min,
            deadline,
        ),
    }
}

/// Return the router/spender address for a given AMM route.
pub fn spender_for_route(route: &AmmSwapRoute) -> Address {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => Address::from([
            0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53, 0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc,
            0xb4, 0xc6, 0x59, 0xF2, 0x48, 0x8D,
        ]),
        AmmSwapRoute::SushiswapV2 { .. } => Address::from([
            0xd9, 0xe1, 0xcE, 0x17, 0xf2, 0x64, 0x1f, 0x24, 0xaE, 0x83, 0x63, 0x7a, 0xb6, 0x6a,
            0x2c, 0xca, 0x9C, 0x37, 0x8B, 0x9F,
        ]),
        AmmSwapRoute::V2Router { router, .. } => router,
        AmmSwapRoute::V3Router { router, .. } => router,
        AmmSwapRoute::UniswapV3 { .. } => Address::from([
            0xE5, 0x92, 0x42, 0x7A, 0x0A, 0xEc, 0xe9, 0x2D, 0xe3, 0xEd, 0xee, 0x1F, 0x18, 0xE0,
            0x15, 0x7C, 0x05, 0x86, 0x15, 0x64,
        ]),
        _ => Address::from([
            0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53, 0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc,
            0xb4, 0xc6, 0x59, 0xF2, 0x48, 0x8D,
        ]),
    }
}

/// Permit payload for single-tx permit + swap flows.
#[derive(Clone, Debug)]
pub struct PermitData {
    pub value: U256,
    pub deadline: u64,
    pub v: u8,
    pub r: [u8; 32],
    pub s: [u8; 32],
}

/// Build a sell swap that bundles a permit when supported.
pub fn build_sell_with_permit(
    route: &AmmSwapRoute,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    permit: PermitData,
    slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => uniswap_v3::build_sell_with_self_permit_v3(
            seller,
            token_in,
            amount_in_tokens,
            fee_tier,
            permit,
            slippage_bps,
            deadline,
        ),
        _ => build_sell_swap(
            route,
            seller,
            token_in,
            amount_in_tokens,
            slippage_bps,
            deadline,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(byte: u8) -> Address {
        Address::from([byte; 20])
    }

    #[test]
    fn v2_denom_buy_uses_supporting_fee_token_to_token_selector() {
        let tx = build_denom_to_token_swap(
            &AmmSwapRoute::UniswapV2 { pool: addr(1) },
            addr(2),
            addr(3),
            addr(4),
            U256::from(1000),
            0,
            u64::MAX,
        );
        let data = tx.data.expect("swap calldata");

        assert_eq!(&data[..4], &[0x5c, 0x11, 0xd7, 0x95]);
        assert_eq!(tx.value, Some(U256::ZERO));
    }

    #[test]
    fn v2_denom_sell_uses_supporting_fee_token_to_token_selector() {
        let tx = build_token_to_denom_swap(
            &AmmSwapRoute::SushiswapV2 { pool: addr(1) },
            addr(2),
            addr(3),
            addr(4),
            U256::from(1000),
            0,
            u64::MAX,
        );
        let data = tx.data.expect("swap calldata");

        assert_eq!(&data[..4], &[0x5c, 0x11, 0xd7, 0x95]);
        assert_eq!(tx.value, Some(U256::ZERO));
    }

    #[test]
    fn v2_router_route_uses_explicit_router_and_spender() {
        let router = addr(9);
        let route = AmmSwapRoute::V2Router {
            pool: addr(1),
            router,
        };
        let tx = build_denom_to_token_swap(
            &route,
            addr(2),
            addr(3),
            addr(4),
            U256::from(1000),
            0,
            u64::MAX,
        );

        assert_eq!(tx.to, Some(router));
        assert_eq!(spender_for_route(&route), router);
    }

    #[test]
    fn v3_denom_buy_uses_exact_input_single_selector() {
        let tx = build_denom_to_token_swap(
            &AmmSwapRoute::UniswapV3 {
                pool: addr(1),
                fee_tier: 3000,
            },
            addr(2),
            addr(3),
            addr(4),
            U256::from(1000),
            0,
            u64::MAX,
        );
        let data = tx.data.expect("swap calldata");

        assert_eq!(&data[..4], &[0x41, 0x4b, 0xf3, 0x89]);
        assert_eq!(tx.value, Some(U256::ZERO));
    }
}
