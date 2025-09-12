pub mod amm;
pub mod amm_swap_route;

use alloy_primitives::{Address, U256};
use tx_simulator::UnsignedTransaction;
use self::amm_swap_route::AmmSwapRoute;

/// Build a buy (ETH -> Token) swap UnsignedTransaction for the given pool spec.
///
/// - buyer: recipient of token_out
/// - token_out: the ERC-20 token being bought
/// - amount_in_eth: ETH amount to spend (in wei)
/// - slippage_bps: slippage in basis points (e.g., 50 = 0.5%). Currently unused (minOut=0)
/// - deadline: unix timestamp deadline; use u64::MAX for no effective deadline
pub fn build_buy_swap(
    route: &AmmSwapRoute,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => amm::v2::build_buy_swap_v2(
            amm::v2::Router::UniswapV2,
            buyer,
            token_out,
            amount_in_eth,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => amm::v2::build_buy_swap_v2(
            amm::v2::Router::SushiswapV2,
            buyer,
            token_out,
            amount_in_eth,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => amm::v3::build_buy_swap_v3(
            buyer,
            token_out,
            amount_in_eth,
            fee_tier,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::CurveV1 { pool, i, j, use_underlying } => amm::curve::build_buy_swap_curve_v1(
            buyer,
            pool,
            i,
            j,
            amount_in_eth,
            use_underlying,
            U256::ZERO,
        ),
        // TODO: Implement Balancer/Fraxswap builders
        _ => amm::v2::build_buy_swap_v2(
            amm::v2::Router::UniswapV2,
            buyer,
            token_out,
            amount_in_eth,
            slippage_bps,
            deadline,
        ),
    }
}

/// Build a buy swap with explicit amountOutMin (slippage enforced by caller).
pub fn build_buy_swap_with_min_out(
    route: &AmmSwapRoute,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => amm::v2::build_buy_swap_v2_with_min_out(
            amm::v2::Router::UniswapV2,
            buyer,
            token_out,
            amount_in_eth,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => amm::v2::build_buy_swap_v2_with_min_out(
            amm::v2::Router::SushiswapV2,
            buyer,
            token_out,
            amount_in_eth,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => amm::v3::build_buy_swap_v3_with_min_out(
            buyer,
            token_out,
            amount_in_eth,
            fee_tier,
            amount_out_min,
            deadline,
        ),
        _ => amm::v2::build_buy_swap_v2_with_min_out(
            amm::v2::Router::UniswapV2,
            buyer,
            token_out,
            amount_in_eth,
            amount_out_min,
            deadline,
        ),
    }
}

/// Build an ERC20 approve transaction to allow the protocol router to spend `amount` of `token`.
pub fn build_approve_for_route(
    route: &AmmSwapRoute,
    owner: Address,
    token: Address,
    amount: U256,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => amm::v2::build_approve_v2(amm::v2::Router::UniswapV2, owner, token, amount),
        AmmSwapRoute::SushiswapV2 { .. } => amm::v2::build_approve_v2(amm::v2::Router::SushiswapV2, owner, token, amount),
        AmmSwapRoute::UniswapV3 { .. } => amm::v3::build_approve_v3(owner, token, amount),
        // Default to V2 router if unknown (can refine when Balancer/Curve supported)
        _ => amm::v2::build_approve_v2(amm::v2::Router::UniswapV2, owner, token, amount),
    }
}

/// Build a sell swap (Token -> ETH/WETH) UnsignedTransaction for the given route.
pub fn build_sell_swap(
    route: &AmmSwapRoute,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => amm::v2::build_sell_swap_v2(
            amm::v2::Router::UniswapV2,
            seller,
            token_in,
            amount_in_tokens,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => amm::v2::build_sell_swap_v2(
            amm::v2::Router::SushiswapV2,
            seller,
            token_in,
            amount_in_tokens,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => amm::v3::build_sell_swap_v3(
            seller,
            token_in,
            amount_in_tokens,
            fee_tier,
            slippage_bps,
            deadline,
        ),
        _ => amm::v2::build_sell_swap_v2(
            amm::v2::Router::UniswapV2,
            seller,
            token_in,
            amount_in_tokens,
            slippage_bps,
            deadline,
        ),
    }
}

/// Build a sell swap with explicit amountOutMin (slippage enforced by caller).
pub fn build_sell_swap_with_min_out(
    route: &AmmSwapRoute,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => amm::v2::build_sell_swap_v2_with_min_out(
            amm::v2::Router::UniswapV2,
            seller,
            token_in,
            amount_in_tokens,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => amm::v2::build_sell_swap_v2_with_min_out(
            amm::v2::Router::SushiswapV2,
            seller,
            token_in,
            amount_in_tokens,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => amm::v3::build_sell_swap_v3_with_min_out(
            seller,
            token_in,
            amount_in_tokens,
            fee_tier,
            amount_out_min,
            deadline,
        ),
        _ => amm::v2::build_sell_swap_v2_with_min_out(
            amm::v2::Router::UniswapV2,
            seller,
            token_in,
            amount_in_tokens,
            amount_out_min,
            deadline,
        ),
    }
}

/// Build a token -> token swap UnsignedTransaction for the given route.
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
        AmmSwapRoute::UniswapV2 { .. } => amm::v2::build_token_to_token_swap_v2(
            amm::v2::Router::UniswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => amm::v2::build_token_to_token_swap_v2(
            amm::v2::Router::SushiswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            slippage_bps,
            deadline,
        ),
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => amm::v3::build_token_to_token_swap_v3(
            trader,
            token_in,
            token_out,
            amount_in,
            fee_tier,
            slippage_bps,
            deadline,
        ),
        _ => amm::v2::build_token_to_token_swap_v2(
            amm::v2::Router::UniswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            slippage_bps,
            deadline,
        ),
    }
}

/// Build a token -> token swap with explicit amountOutMin (slippage enforced by caller).
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
        AmmSwapRoute::UniswapV2 { .. } => amm::v2::build_token_to_token_swap_v2_with_min_out(
            amm::v2::Router::UniswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::SushiswapV2 { .. } => amm::v2::build_token_to_token_swap_v2_with_min_out(
            amm::v2::Router::SushiswapV2,
            trader,
            token_in,
            token_out,
            amount_in,
            amount_out_min,
            deadline,
        ),
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => amm::v3::build_token_to_token_swap_v3_with_min_out(
            trader,
            token_in,
            token_out,
            amount_in,
            fee_tier,
            amount_out_min,
            deadline,
        ),
        _ => amm::v2::build_token_to_token_swap_v2_with_min_out(
            amm::v2::Router::UniswapV2,
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
/// Used when checking ERC20 allowance for sells.
pub fn spender_for_route(route: &AmmSwapRoute) -> Address {
    match *route {
        AmmSwapRoute::UniswapV2 { .. } => Address::from([
            0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53,
            0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc, 0xb4, 0xc6,
            0x59, 0xF2, 0x48, 0x8D,
        ]),
        AmmSwapRoute::SushiswapV2 { .. } => Address::from([
            0xd9, 0xe1, 0xcE, 0x17, 0xf2, 0x64, 0x1f, 0x24,
            0xaE, 0x83, 0x63, 0x7a, 0xb6, 0x6a, 0x2c, 0xca,
            0x9C, 0x37, 0x8B, 0x9F,
        ]),
        AmmSwapRoute::UniswapV3 { .. } => Address::from([
            0xE5, 0x92, 0x42, 0x7A, 0x0A, 0xEc, 0xe9, 0x2D,
            0xe3, 0xEd, 0xee, 0x1F, 0x18, 0xE0, 0x15, 0x7C,
            0x05, 0x86, 0x15, 0x64,
        ]),
        // Defaults for not-yet-implemented protocols
        _ => Address::from([
            0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53,
            0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc, 0xb4, 0xc6,
            0x59, 0xF2, 0x48, 0x8D,
        ]),
    }
}

/// Permit payload for single-tx permit + swap flows.
#[derive(Clone, Debug)]
pub struct PermitData {
    pub value: U256,     // allowance value to permit
    pub deadline: u64,   // permit deadline (unix ts)
    pub v: u8,
    pub r: [u8; 32],
    pub s: [u8; 32],
}

/// Build a sell swap that attempts to bundle permit approval into the same transaction
/// when supported by the target route. Currently only Uniswap V3 routes are candidates
/// (via SwapRouter.selfPermit + multicall). For other routes this falls back to a standard
/// sell swap without permit bundling.
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
        AmmSwapRoute::UniswapV3 { fee_tier, .. } => amm::v3::build_sell_with_self_permit_v3(
            seller,
            token_in,
            amount_in_tokens,
            fee_tier,
            permit,
            slippage_bps,
            deadline,
        ),
        _ => build_sell_swap(route, seller, token_in, amount_in_tokens, slippage_bps, deadline),
    }
}
