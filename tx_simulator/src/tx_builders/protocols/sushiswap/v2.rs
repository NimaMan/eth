use crate::tx_builders::protocols::uniswap::v2::Router;
use alloy_primitives::{address, Address};

pub use crate::tx_builders::protocols::uniswap::v2::{
    build_approve_v2, build_buy_swap_v2, build_buy_swap_v2_with_min_out,
    build_buy_swap_v2_with_min_out_path, build_buy_swap_v2_with_path, build_sell_swap_v2,
    build_sell_swap_v2_with_min_out, build_sell_swap_v2_with_min_out_path,
    build_sell_swap_v2_with_path, build_token_to_token_swap_supporting_fee_v2,
    build_token_to_token_swap_v2, build_token_to_token_swap_v2_with_min_out,
};

pub const DEFAULT_ROUTER_ADDRESS: Address = address!("d9e1cE17f2641f24aE83637ab66a2cca9C378B9F");
pub const DEFAULT_ROUTER: Router = Router::Custom(DEFAULT_ROUTER_ADDRESS);
