use crate::tx_builders::PermitData;
use crate::UnsignedTransaction;
use alloy_primitives::{Address, Bytes, U256};

fn router_address_v3() -> Address {
    Address::from([
        0xE5, 0x92, 0x42, 0x7A, 0x0A, 0xEc, 0xe9, 0x2D, 0xe3, 0xEd, 0xee, 0x1F, 0x18, 0xE0, 0x15,
        0x7C, 0x05, 0x86, 0x15, 0x64,
    ])
}

fn weth_address() -> Address {
    Address::from([
        0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9,
        0x08, 0x3C, 0x75, 0x6C, 0xc2,
    ])
}

/// Encode exactInputSingle(params)
fn encode_exact_input_single(
    token_in: Address,
    token_out: Address,
    fee: u32,
    recipient: Address,
    deadline: U256,
    amount_in: U256,
    amount_out_minimum: U256,
    sqrt_price_limit_x96: U256,
) -> Bytes {
    // Selector for exactInputSingle: 0x414bf389
    let mut data = vec![0x41, 0x4b, 0xf3, 0x89];

    // tokenIn (32 bytes, padded)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(token_in.as_slice());

    // tokenOut
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(token_out.as_slice());

    // fee (uint24 right-aligned in 32 bytes). We'll write 3 bytes from fee.
    data.extend_from_slice(&[0u8; 29]);
    let fee_bytes = fee.to_be_bytes(); // 4 bytes
    data.extend_from_slice(&fee_bytes[1..]);

    // recipient
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(recipient.as_slice());

    // deadline
    data.extend_from_slice(&deadline.to_be_bytes::<32>());

    // amountIn
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());

    // amountOutMinimum
    data.extend_from_slice(&amount_out_minimum.to_be_bytes::<32>());

    // sqrtPriceLimitX96
    data.extend_from_slice(&sqrt_price_limit_x96.to_be_bytes::<32>());

    Bytes::from(data)
}

/// Build a Uniswap V3 buy swap (ETH -> token) via SwapRouter exactInputSingle.
pub fn build_buy_swap_v3(
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    fee_tier: u32,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_buy_swap_v3_with_router(
        router_address_v3(),
        buyer,
        token_out,
        amount_in_eth,
        fee_tier,
        deadline,
    )
}

pub fn build_buy_swap_v3_with_router(
    router: Address,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    fee_tier: u32,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_exact_input_single(
        weth_address(),
        token_out,
        fee_tier,
        buyer,
        U256::from(deadline),
        amount_in_eth,
        U256::ZERO, // accept-any for testing
        U256::ZERO, // no price limit
    );

    UnsignedTransaction {
        from: Some(buyer),
        to: Some(router),
        gas: Some(350_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(amount_in_eth), // SwapRouter handles WETH wrapping
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Build a Uniswap V3 buy swap with explicit amountOutMinimum.
pub fn build_buy_swap_v3_with_min_out(
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_buy_swap_v3_with_min_out_router(
        router_address_v3(),
        buyer,
        token_out,
        amount_in_eth,
        fee_tier,
        amount_out_min,
        deadline,
    )
}

pub fn build_buy_swap_v3_with_min_out_router(
    router: Address,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_exact_input_single(
        weth_address(),
        token_out,
        fee_tier,
        buyer,
        U256::from(deadline),
        amount_in_eth,
        amount_out_min,
        U256::ZERO,
    );

    UnsignedTransaction {
        from: Some(buyer),
        to: Some(router),
        gas: Some(350_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(amount_in_eth),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// ERC20 approve(selector 0x095ea7b3)
fn encode_approve(spender: Address, amount: U256) -> Bytes {
    let mut data = vec![0x09, 0x5e, 0xa7, 0xb3];
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(spender.as_slice());
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    Bytes::from(data)
}

/// Build approve(tx) for V3 SwapRouter as spender.
pub fn build_approve_v3(owner: Address, token: Address, amount: U256) -> UnsignedTransaction {
    build_approve_v3_for_router(router_address_v3(), owner, token, amount)
}

pub fn build_approve_v3_for_router(
    router: Address,
    owner: Address,
    token: Address,
    amount: U256,
) -> UnsignedTransaction {
    let calldata = encode_approve(router, amount);
    UnsignedTransaction {
        from: Some(owner),
        to: Some(token),
        gas: Some(120_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Build a Uniswap V3 sell swap (Token -> WETH) via SwapRouter exactInputSingle.
pub fn build_sell_swap_v3(
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_sell_swap_v3_with_router(
        router_address_v3(),
        seller,
        token_in,
        amount_in_tokens,
        fee_tier,
        deadline,
    )
}

pub fn build_sell_swap_v3_with_router(
    router: Address,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_exact_input_single(
        token_in,
        weth_address(),
        fee_tier,
        seller,
        U256::from(deadline),
        amount_in_tokens,
        U256::ZERO, // accept-any for testing
        U256::ZERO, // no price limit
    );

    UnsignedTransaction {
        from: Some(seller),
        to: Some(router),
        gas: Some(350_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Build a Uniswap V3 sell swap with explicit amountOutMinimum.
pub fn build_sell_swap_v3_with_min_out(
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_sell_swap_v3_with_min_out_router(
        router_address_v3(),
        seller,
        token_in,
        amount_in_tokens,
        fee_tier,
        amount_out_min,
        deadline,
    )
}

pub fn build_sell_swap_v3_with_min_out_router(
    router: Address,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_exact_input_single(
        token_in,
        weth_address(),
        fee_tier,
        seller,
        U256::from(deadline),
        amount_in_tokens,
        amount_out_min,
        U256::ZERO,
    );

    UnsignedTransaction {
        from: Some(seller),
        to: Some(router),
        gas: Some(350_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Build a Uniswap V3 token -> token swap (exactInputSingle).
pub fn build_token_to_token_swap_v3(
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    build_token_to_token_swap_v3_with_router(
        router_address_v3(),
        trader,
        token_in,
        token_out,
        amount_in,
        fee_tier,
        deadline,
    )
}

pub fn build_token_to_token_swap_v3_with_router(
    router: Address,
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_exact_input_single(
        token_in,
        token_out,
        fee_tier,
        trader,
        U256::from(deadline),
        amount_in,
        U256::ZERO,
        U256::ZERO,
    );

    UnsignedTransaction {
        from: Some(trader),
        to: Some(router),
        gas: Some(350_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Build a Uniswap V3 token -> token swap with explicit amountOutMinimum.
pub fn build_token_to_token_swap_v3_with_min_out(
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    build_token_to_token_swap_v3_with_min_out_router(
        router_address_v3(),
        trader,
        token_in,
        token_out,
        amount_in,
        fee_tier,
        amount_out_min,
        deadline,
    )
}

pub fn build_token_to_token_swap_v3_with_min_out_router(
    router: Address,
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    fee_tier: u32,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_exact_input_single(
        token_in,
        token_out,
        fee_tier,
        trader,
        U256::from(deadline),
        amount_in,
        amount_out_min,
        U256::ZERO,
    );

    UnsignedTransaction {
        from: Some(trader),
        to: Some(router),
        gas: Some(350_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Build a Uniswap V3 sell swap that attempts to include a selfPermit via multicall.
/// NOTE: Placeholder implementation currently falls back to a standard sell swap.
/// Proper selfPermit + multicall encoding will be added in a subsequent pass.
pub fn build_sell_with_self_permit_v3(
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    fee_tier: u32,
    _permit: PermitData,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    // Fallback: build standard sell (no permit bundling yet)
    build_sell_swap_v3(seller, token_in, amount_in_tokens, fee_tier, 0, deadline)
}
