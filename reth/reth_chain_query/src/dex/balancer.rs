//! Balancer pool helpers.

use crate::dex::encoding::{encode_bytes32, encode_function_call};
use crate::TxSimulator;
use alloy_primitives::{address, Address, U256};
use eyre::Result;

/// Balancer Vault contract address (for dynamic pool discovery).
pub const BALANCER_VAULT: Address = address!("BA12222222228d8Ba445958a75a0704d566BF2C8");

const BALANCER_GET_POOL_TOKENS: [u8; 4] = [0xf9, 0x4d, 0x46, 0x68];

/// Verify Balancer pool contains specific token pair.
pub async fn verify_balancer_pool_tokens(
    simulator: &TxSimulator,
    pool_id: [u8; 32],
    token_a: Address,
    token_b: Address,
    block_number: Option<u64>,
) -> Result<bool> {
    let params = encode_bytes32(pool_id);
    let call_data = encode_function_call(BALANCER_GET_POOL_TOKENS, &params);

    let result = simulator
        .simulate_view_function(BALANCER_VAULT, call_data, block_number, None)
        .await?;

    if result.success {
        if let Some(tokens) = decode_address_array_from_result(&result.output) {
            let has_token_a = tokens.contains(&token_a);
            let has_token_b = tokens.contains(&token_b);
            return Ok(has_token_a && has_token_b);
        }
    }

    Ok(false)
}

/// Get all tokens in a Balancer pool.
pub async fn get_balancer_pool_tokens(
    simulator: &TxSimulator,
    pool_id: [u8; 32],
    block_number: Option<u64>,
) -> Result<Option<(Vec<Address>, Vec<U256>)>> {
    let params = encode_bytes32(pool_id);
    let call_data = encode_function_call(BALANCER_GET_POOL_TOKENS, &params);

    let result = simulator
        .simulate_view_function(BALANCER_VAULT, call_data, block_number, None)
        .await?;

    if result.success {
        if let (Some(tokens), Some(balances)) = (
            decode_address_array_from_result(&result.output),
            decode_uint256_array_from_result(&result.output),
        ) {
            return Ok(Some((tokens, balances)));
        }
    }

    Ok(None)
}

fn decode_address_array_from_result(result: &[u8]) -> Option<Vec<Address>> {
    if result.len() < 96 {
        return None;
    }
    let tokens_off = be_word_to_usize(&result[0..32]);
    if tokens_off == 0 || result.len() < tokens_off + 32 {
        return None;
    }
    let len = be_word_to_usize(&result[tokens_off..tokens_off + 32]);
    let mut out = Vec::with_capacity(len);
    let mut cur = tokens_off + 32;
    for _ in 0..len {
        if result.len() < cur + 32 {
            return None;
        }
        out.push(Address::from_slice(&result[cur + 12..cur + 32]));
        cur += 32;
    }
    Some(out)
}

fn decode_uint256_array_from_result(result: &[u8]) -> Option<Vec<U256>> {
    if result.len() < 64 {
        return None;
    }
    let balances_off = be_word_to_usize(&result[32..64]);
    if balances_off == 0 || result.len() < balances_off + 32 {
        return None;
    }
    let len = be_word_to_usize(&result[balances_off..balances_off + 32]);
    let mut out = Vec::with_capacity(len);
    let mut cur = balances_off + 32;
    for _ in 0..len {
        if result.len() < cur + 32 {
            return None;
        }
        out.push(U256::from_be_slice(&result[cur..cur + 32]));
        cur += 32;
    }
    Some(out)
}

fn be_word_to_usize(word: &[u8]) -> usize {
    if word.len() < 32 {
        return 0;
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&word[24..32]);
    usize::from_be_bytes(bytes)
}
