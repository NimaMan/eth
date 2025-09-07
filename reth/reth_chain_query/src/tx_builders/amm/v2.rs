use alloy_primitives::{Address, Bytes, U256};
use tx_simulator::UnsignedTransaction;

#[derive(Debug, Clone, Copy)]
pub enum Router {
    UniswapV2,
    SushiswapV2,
}

fn router_address(router: Router) -> Address {
    match router {
        Router::UniswapV2 => Address::from([
            0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53,
            0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc, 0xb4, 0xc6,
            0x59, 0xF2, 0x48, 0x8D,
        ]),
        Router::SushiswapV2 => Address::from([
            0xd9, 0xe1, 0xcE, 0x17, 0xf2, 0x64, 0x1f, 0x24,
            0xaE, 0x83, 0x63, 0x7a, 0xb6, 0x6a, 0x2c, 0xca,
            0x9C, 0x37, 0x8B, 0x9F,
        ]),
    }
}

fn weth_address() -> Address {
    Address::from([
        0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D,
        0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08,
        0x3C, 0x75, 0x6C, 0xc2,
    ])
}

/// Encode swapExactETHForTokens(amountOutMin, path, to, deadline)
fn encode_swap_exact_eth_for_tokens(
    amount_out_min: U256,
    path: &[Address; 2],
    to: Address,
    deadline: U256,
) -> Bytes {
    // Function selector: 0x7ff36ab5
    let mut data = vec![0x7f, 0xf3, 0x6a, 0xb5];

    // amountOutMin
    data.extend_from_slice(&amount_out_min.to_be_bytes::<32>());

    // path offset (dynamic array) -> 0x80
    data.extend_from_slice(&[0u8; 28]);
    data.extend_from_slice(&[0, 0, 0, 0x80]);

    // to address (padded)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(to.as_slice());

    // deadline
    data.extend_from_slice(&deadline.to_be_bytes::<32>());

    // path array header
    data.extend_from_slice(&[0u8; 28]);
    data.extend_from_slice(&[0, 0, 0, 0x02]); // length = 2

    // path[0]
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(path[0].as_slice());
    // path[1]
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(path[1].as_slice());

    Bytes::from(data)
}

/// Build a Uniswap/Sushiswap V2 buy swap (ETH -> token) via router.
pub fn build_buy_swap_v2(
    router: Router,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_swap_exact_eth_for_tokens(
        U256::ZERO,                              // accept-any for testing; compute minOut if needed
        &[weth_address(), token_out],            // WETH -> token
        buyer,                                   // recipient
        U256::from(deadline),
    );

    UnsignedTransaction {
        from: Some(buyer),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: Some(100_000_000_000),
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(amount_in_eth),
        data: Some(calldata),
        nonce: None,
    }
}

/// Build a Uniswap/Sushiswap V2 buy swap with explicit amountOutMin.
pub fn build_buy_swap_v2_with_min_out(
    router: Router,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_swap_exact_eth_for_tokens(
        amount_out_min,
        &[weth_address(), token_out],
        buyer,
        U256::from(deadline),
    );

    UnsignedTransaction {
        from: Some(buyer),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: Some(100_000_000_000),
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(amount_in_eth),
        data: Some(calldata),
        nonce: None,
    }
}

/// Encode swapExactTokensForETH(amountIn, amountOutMin, path, to, deadline)
fn encode_swap_exact_tokens_for_eth(
    amount_in: U256,
    amount_out_min: U256,
    path: &[Address; 2],
    to: Address,
    deadline: U256,
) -> Bytes {
    // Selector: 0x18cbafe5
    let mut data = vec![0x18, 0xcb, 0xaf, 0xe5];

    // amountIn
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());

    // amountOutMin
    data.extend_from_slice(&amount_out_min.to_be_bytes::<32>());

    // path offset -> 0xa0
    data.extend_from_slice(&[0u8; 28]);
    data.extend_from_slice(&[0, 0, 0, 0xa0]);

    // to address
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(to.as_slice());

    // deadline
    data.extend_from_slice(&deadline.to_be_bytes::<32>());

    // path array header
    data.extend_from_slice(&[0u8; 28]);
    data.extend_from_slice(&[0, 0, 0, 0x02]);

    // path[0] = token_in
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(path[0].as_slice());
    // path[1] = WETH
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(path[1].as_slice());

    Bytes::from(data)
}

/// ERC20 approve(selector 0x095ea7b3)
fn encode_approve(spender: Address, amount: U256) -> Bytes {
    let mut data = vec![0x09, 0x5e, 0xa7, 0xb3];
    // spender
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(spender.as_slice());
    // amount
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    Bytes::from(data)
}

/// Build approve(tx) for V2/Sushiswap router as spender.
pub fn build_approve_v2(router: Router, owner: Address, token: Address, amount: U256) -> UnsignedTransaction {
    let calldata = encode_approve(router_address(router), amount);
    UnsignedTransaction {
        from: Some(owner),
        to: Some(token),
        gas: Some(100_000),
        gas_price: Some(100_000_000_000),
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
    }
}

/// Build a Uniswap/Sushiswap V2 sell swap (Token -> ETH) via router.
pub fn build_sell_swap_v2(
    router: Router,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_swap_exact_tokens_for_eth(
        amount_in_tokens,
        U256::ZERO,                     // accept-any for testing
        &[token_in, weth_address()],     // token -> WETH
        seller,
        U256::from(deadline),
    );

    UnsignedTransaction {
        from: Some(seller),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: Some(100_000_000_000),
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
    }
}

/// Build a Uniswap/Sushiswap V2 sell swap with explicit amountOutMin.
pub fn build_sell_swap_v2_with_min_out(
    router: Router,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_swap_exact_tokens_for_eth(
        amount_in_tokens,
        amount_out_min,
        &[token_in, weth_address()],
        seller,
        U256::from(deadline),
    );

    UnsignedTransaction {
        from: Some(seller),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: Some(100_000_000_000),
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
    }
}
