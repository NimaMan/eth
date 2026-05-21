use crate::UnsignedTransaction;
use alloy_primitives::{Address, Bytes, U256};

#[derive(Debug, Clone, Copy)]
pub enum Router {
    UniswapV2,
    Custom(Address),
}

fn router_address(router: Router) -> Address {
    match router {
        Router::UniswapV2 => Address::from([
            0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53, 0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc,
            0xb4, 0xc6, 0x59, 0xF2, 0x48, 0x8D,
        ]),
        Router::Custom(router) => router,
    }
}

fn weth_address() -> Address {
    Address::from([
        0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9,
        0x08, 0x3C, 0x75, 0x6C, 0xc2,
    ])
}

fn encode_dynamic_address_array(path: &[Address]) -> Vec<u8> {
    assert!(
        path.len() >= 2,
        "swap path must contain at least input and output tokens"
    );
    let mut encoded = Vec::with_capacity(32 + path.len() * 32);

    encoded.extend_from_slice(&U256::from(path.len() as u64).to_be_bytes::<32>());

    for address in path {
        encoded.extend_from_slice(&[0u8; 12]);
        encoded.extend_from_slice(address.as_slice());
    }

    encoded
}

/// Encode swapExactETHForTokens(amountOutMin, path, to, deadline)
fn encode_swap_exact_eth_for_tokens(
    amount_out_min: U256,
    path: &[Address],
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

    // path dynamic array
    data.extend_from_slice(&encode_dynamic_address_array(path));

    Bytes::from(data)
}

/// Encode swapExactETHForTokensSupportingFeeOnTransferTokens(amountOutMin, path, to, deadline)
fn encode_swap_exact_eth_for_tokens_supporting_fee(
    amount_out_min: U256,
    path: &[Address],
    to: Address,
    deadline: U256,
) -> Bytes {
    // Function selector: 0xb6f9de95
    let mut data = vec![0xb6, 0xf9, 0xde, 0x95];

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

    // path dynamic array
    data.extend_from_slice(&encode_dynamic_address_array(path));

    Bytes::from(data)
}

/// Build a V2-style buy swap (ETH -> token) via router.
fn default_buy_path(token_out: Address) -> [Address; 2] {
    [weth_address(), token_out]
}

fn default_sell_path(token_in: Address) -> [Address; 2] {
    [token_in, weth_address()]
}

pub fn build_buy_swap_v2_with_path(
    router: Router,
    buyer: Address,
    amount_in_eth: U256,
    path: &[Address],
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_swap_exact_eth_for_tokens(
        U256::ZERO, // accept-any for testing; compute minOut if needed
        path,
        buyer, // recipient
        U256::from(deadline),
    );

    UnsignedTransaction {
        from: Some(buyer),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(amount_in_eth),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

pub fn build_buy_swap_v2(
    router: Router,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    let path = default_buy_path(token_out);
    build_buy_swap_v2_with_path(router, buyer, amount_in_eth, &path, deadline)
}

/// Build a V2-style buy swap with explicit amountOutMin.
pub fn build_buy_swap_v2_with_min_out(
    router: Router,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let path = default_buy_path(token_out);
    build_buy_swap_v2_with_min_out_path(
        router,
        buyer,
        amount_in_eth,
        &path,
        amount_out_min,
        deadline,
    )
}

pub fn build_buy_swap_v2_with_min_out_path(
    router: Router,
    buyer: Address,
    amount_in_eth: U256,
    path: &[Address],
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata =
        encode_swap_exact_eth_for_tokens(amount_out_min, path, buyer, U256::from(deadline));

    UnsignedTransaction {
        from: Some(buyer),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(amount_in_eth),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Build a V2-style buy swap through the supporting-fee-on-transfer router
/// method. This matches the UniswapV2TradingVault buy path.
pub fn build_buy_swap_v2_supporting_fee_with_min_out(
    router: Router,
    buyer: Address,
    token_out: Address,
    amount_in_eth: U256,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let path = default_buy_path(token_out);
    build_buy_swap_v2_supporting_fee_with_min_out_path(
        router,
        buyer,
        amount_in_eth,
        &path,
        amount_out_min,
        deadline,
    )
}

pub fn build_buy_swap_v2_supporting_fee_with_min_out_path(
    router: Router,
    buyer: Address,
    amount_in_eth: U256,
    path: &[Address],
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_swap_exact_eth_for_tokens_supporting_fee(
        amount_out_min,
        path,
        buyer,
        U256::from(deadline),
    );

    UnsignedTransaction {
        from: Some(buyer),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(amount_in_eth),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Encode swapExactTokensForETHSupportingFeeOnTransferTokens(amountIn, amountOutMin, path, to, deadline)
fn encode_swap_exact_tokens_for_eth(
    amount_in: U256,
    amount_out_min: U256,
    path: &[Address],
    to: Address,
    deadline: U256,
) -> Bytes {
    // Selector: 0x791ac947
    let mut data = vec![0x79, 0x1a, 0xc9, 0x47];

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

    // path dynamic array
    data.extend_from_slice(&encode_dynamic_address_array(path));

    Bytes::from(data)
}

/// Encode swapExactTokensForTokens(amountIn, amountOutMin, path, to, deadline)
fn encode_swap_exact_tokens_for_tokens(
    amount_in: U256,
    amount_out_min: U256,
    path: &[Address],
    to: Address,
    deadline: U256,
) -> Bytes {
    // Selector: 0x38ed1739
    let mut data = vec![0x38, 0xed, 0x17, 0x39];

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

    // path dynamic array
    data.extend_from_slice(&encode_dynamic_address_array(path));

    Bytes::from(data)
}

/// Encode swapExactTokensForTokensSupportingFeeOnTransferTokens(amountIn, amountOutMin, path, to, deadline)
fn encode_swap_exact_tokens_for_tokens_supporting_fee(
    amount_in: U256,
    amount_out_min: U256,
    path: &[Address],
    to: Address,
    deadline: U256,
) -> Bytes {
    // Selector: 0x5c11d795
    let mut data = vec![0x5c, 0x11, 0xd7, 0x95];

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

    // path dynamic array
    data.extend_from_slice(&encode_dynamic_address_array(path));

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

/// Build approve(tx) for a V2-style router as spender.
pub fn build_approve_v2(
    router: Router,
    owner: Address,
    token: Address,
    amount: U256,
) -> UnsignedTransaction {
    let calldata = encode_approve(router_address(router), amount);
    UnsignedTransaction {
        from: Some(owner),
        to: Some(token),
        gas: Some(100_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Build a V2-style sell swap (Token -> ETH) via router.
pub fn build_sell_swap_v2(
    router: Router,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    let path = default_sell_path(token_in);
    build_sell_swap_v2_with_path(router, seller, amount_in_tokens, &path, deadline)
}

/// Build a V2-style sell swap with explicit amountOutMin.
pub fn build_sell_swap_v2_with_min_out(
    router: Router,
    seller: Address,
    token_in: Address,
    amount_in_tokens: U256,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let path = default_sell_path(token_in);
    build_sell_swap_v2_with_min_out_path(
        router,
        seller,
        amount_in_tokens,
        &path,
        amount_out_min,
        deadline,
    )
}

pub fn build_sell_swap_v2_with_path(
    router: Router,
    seller: Address,
    amount_in_tokens: U256,
    path: &[Address],
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_swap_exact_tokens_for_eth(
        amount_in_tokens,
        U256::ZERO, // accept-any for testing
        path,
        seller,
        U256::from(deadline),
    );

    UnsignedTransaction {
        from: Some(seller),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

pub fn build_sell_swap_v2_with_min_out_path(
    router: Router,
    seller: Address,
    amount_in_tokens: U256,
    path: &[Address],
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_swap_exact_tokens_for_eth(
        amount_in_tokens,
        amount_out_min,
        path,
        seller,
        U256::from(deadline),
    );

    UnsignedTransaction {
        from: Some(seller),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Build a V2-style token -> token swap via router.
pub fn build_token_to_token_swap_v2(
    router: Router,
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    _slippage_bps: u32,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_swap_exact_tokens_for_tokens(
        amount_in,
        U256::ZERO,
        &[token_in, token_out],
        trader,
        U256::from(deadline),
    );

    UnsignedTransaction {
        from: Some(trader),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Build a V2-style token -> token swap that tolerates fee-on-transfer tokens.
pub fn build_token_to_token_swap_supporting_fee_v2(
    router: Router,
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_swap_exact_tokens_for_tokens_supporting_fee(
        amount_in,
        U256::ZERO,
        &[token_in, token_out],
        trader,
        U256::from(deadline),
    );

    UnsignedTransaction {
        from: Some(trader),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

/// Build a V2-style token -> token swap with explicit amountOutMin.
pub fn build_token_to_token_swap_v2_with_min_out(
    router: Router,
    trader: Address,
    token_in: Address,
    token_out: Address,
    amount_in: U256,
    amount_out_min: U256,
    deadline: u64,
) -> UnsignedTransaction {
    let calldata = encode_swap_exact_tokens_for_tokens(
        amount_in,
        amount_out_min,
        &[token_in, token_out],
        trader,
        U256::from(deadline),
    );

    UnsignedTransaction {
        from: Some(trader),
        to: Some(router_address(router)),
        gas: Some(500_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::keccak256;

    #[test]
    fn encodes_supporting_fee_buy_selector_and_fields() {
        let token = Address::with_last_byte(0x22);
        let path = default_buy_path(token);
        let data = encode_swap_exact_eth_for_tokens_supporting_fee(
            U256::from(123),
            &path,
            Address::with_last_byte(0x33),
            U256::from(1_800_000_000u64),
        );
        let selector = keccak256(
            "swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256)"
                .as_bytes(),
        );

        assert_eq!(&data[..4], &selector[..4]);
        assert_eq!(data.len(), 4 + 32 * 4 + 32 + 32 * 2);
    }

    #[test]
    fn builds_supporting_fee_buy_to_router() {
        let buyer = Address::with_last_byte(0x44);
        let token = Address::with_last_byte(0x55);
        let tx = build_buy_swap_v2_supporting_fee_with_min_out(
            Router::UniswapV2,
            buyer,
            token,
            U256::from(1_000_000_000_000_000u128),
            U256::from(1),
            1_800_000_000,
        );

        assert_eq!(tx.from, Some(buyer));
        assert_eq!(tx.to, Some(router_address(Router::UniswapV2)));
        assert_eq!(tx.value, Some(U256::from(1_000_000_000_000_000u128)));
        assert_eq!(tx.gas, Some(500_000));
        assert_eq!(&tx.data.expect("calldata")[..4], &[0xb6, 0xf9, 0xde, 0x95]);
    }
}
