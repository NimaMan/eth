use crate::tx_builders::protocols::v3_swap_router::{
    self, ExactInputSingleAbi, ExactInputSingleRequest, WETH_ADDRESS,
};
use crate::UnsignedTransaction;
use alloy_primitives::{address, Address, U256};

pub const DEFAULT_ROUTER: Address = address!("2E6cd2d30aa43f40aa81619ff4b6E0a41479B13F");
const SWAP_ROUTER_ABI: ExactInputSingleAbi = ExactInputSingleAbi::WithDeadline;

pub fn build_buy_swap_v3(
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    fee_tier: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_exact_input_single(
        buyer,
        WETH_ADDRESS,
        token_out,
        amount_in_eth,
        fee_tier,
        buyer,
        U256::ZERO,
        U256::from(deadline),
        amount_in_eth,
    )
}

pub fn build_buy_swap_v3_with_min_out(
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_exact_input_single(
        buyer,
        WETH_ADDRESS,
        token_out,
        amount_in_eth,
        fee_tier,
        buyer,
        amount_out_min,
        U256::from(deadline),
        amount_in_eth,
    )
}

pub fn build_approve_v3(owner: Address, token: Address, amount: U256) -> UnsignedTransaction {
    v3_swap_router::build_approve_for_router(DEFAULT_ROUTER, owner, token, amount)
}

pub fn build_sell_swap_v3(
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_exact_input_single(
        seller,
        token_in,
        WETH_ADDRESS,
        amount_in_tokens,
        fee_tier,
        seller,
        U256::ZERO,
        U256::from(deadline),
        U256::ZERO,
    )
}

pub fn build_sell_swap_v3_with_min_out(
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_exact_input_single(
        seller,
        token_in,
        WETH_ADDRESS,
        amount_in_tokens,
        fee_tier,
        seller,
        amount_out_min,
        U256::from(deadline),
        U256::ZERO,
    )
}

pub fn build_token_to_token_swap_v3(
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_token_to_token_swap_v3_with_min_out(
        trader,
        token_in,
        token_out,
        amount_in,
        fee_tier,
        U256::ZERO,
        deadline,
    )
}

pub fn build_token_to_token_swap_v3_with_min_out(
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_exact_input_single(
        trader,
        token_in,
        token_out,
        amount_in,
        fee_tier,
        trader,
        amount_out_min,
        U256::from(deadline),
        U256::ZERO,
    )
}

fn build_exact_input_single(
    caller: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    recipient: Address,
    amount_out_minimum: U256,
    deadline: U256,
    value: U256,
) -> UnsignedTransaction {
    v3_swap_router::build_exact_input_single_tx(ExactInputSingleRequest {
        router: DEFAULT_ROUTER,
        caller,
        token_in,
        token_out,
        fee: fee_tier,
        recipient,
        deadline,
        amount_in,
        amount_out_minimum,
        sqrt_price_limit_x96: U256::ZERO,
        value,
        abi: SWAP_ROUTER_ABI,
    })
}
