use alloy_primitives::{keccak256, Address, Bytes, U256};
use tx_simulator::UnsignedTransaction;

/// Encode Curve V1 exchange or exchange_underlying call data.
fn encode_curve_exchange(i: u8, j: u8, dx: U256, min_dy: U256, use_underlying: bool) -> Bytes {
    let sig = if use_underlying {
        "exchange_underlying(int128,int128,uint256,uint256)"
    } else {
        "exchange(int128,int128,uint256,uint256)"
    };
    let hash = keccak256(sig.as_bytes());
    let selector = &hash.as_slice()[0..4];
    let mut data = Vec::with_capacity(4 + 32 * 4);
    data.extend_from_slice(selector);

    // int128 i
    let mut i_bytes = [0u8; 32];
    i_bytes[31] = i; // positive small int fits in last byte
    data.extend_from_slice(&i_bytes);

    // int128 j
    let mut j_bytes = [0u8; 32];
    j_bytes[31] = j;
    data.extend_from_slice(&j_bytes);

    // uint256 dx
    data.extend_from_slice(&dx.to_be_bytes::<32>());

    // uint256 min_dy
    data.extend_from_slice(&min_dy.to_be_bytes::<32>());

    Bytes::from(data)
}

/// Build a Curve V1 buy-swap (ETH -> token) transaction by calling exchange/exchange_underlying.
///
/// Notes:
/// - This path assumes the input coin at index `i` accepts native ETH (e.g., stETH/ETH pool).
/// - For WETH-based pools (e.g., TriCrypto), a two-step wrap + exchange is required (not handled here).
pub fn build_buy_swap_curve_v1(
    buyer: Address,
    pool: Address,
    i: u8,
    j: u8,
    amount_in_eth: U256,
    use_underlying: bool,
    min_dy: U256,
) -> UnsignedTransaction {
    let calldata = encode_curve_exchange(i, j, amount_in_eth, min_dy, use_underlying);
    UnsignedTransaction {
        from: Some(buyer),
        to: Some(pool),
        gas: Some(500_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        // Send ETH as input amount
        value: Some(amount_in_eth),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}
