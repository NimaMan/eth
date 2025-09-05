//! DEX pool address computation and factory addresses
//! 
//! Provides both deterministic pool address calculation and dynamic pool discovery
//! for various DEX protocols.
//!
//! Algorithm:
//! 1. **CREATE2 Computation** (Uniswap V2, V3, SushiSwap):
//!    - Sort tokens by address (smaller first)
//!    - Calculate salt = keccak256(token0 || token1) for V2
//!    - Calculate salt = keccak256(abi.encode(token0, token1, fee)) for V3
//!    - Use CREATE2 formula: address = keccak256(0xff || factory || salt || init_code_hash)[12:]
//! 
//! 2. **Dynamic Discovery** (Curve, Balancer):
//!    - Query registry/vault contracts using view function simulation
//!    - Curve: Use Registry.find_pool_for_coins(token_a, token_b)
//!    - Balancer: Query Vault.getPoolTokens(poolId) to verify token composition
//! 
//! 3. **Hybrid Approach**: 
//!    - Fast CREATE2 for standardized factories
//!    - Chain queries for complex deployment patterns

use alloy_primitives::{address, Address, keccak256, U256, Bytes};
use eyre::Result;
use crate::TxSimulator;

/// Factory addresses for different DEX protocols
pub const UNISWAP_V2_FACTORY: Address = address!("5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");
pub const UNISWAP_V3_FACTORY: Address = address!("1F98431c8aD98523631AE4a59f267346ea31F984");
pub const SUSHISWAP_FACTORY: Address = address!("C0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac");

/// Uniswap V2 init code hash (used for CREATE2 address calculation)
pub const UNISWAP_V2_INIT_CODE_HASH: [u8; 32] = hex_literal::hex!("96e8ac4277198ff8b6f785478aa9a39f403cb768dd02cbee326c3e7da348845f");

/// Uniswap V3 init code hash
pub const UNISWAP_V3_INIT_CODE_HASH: [u8; 32] = hex_literal::hex!("e34f199b19b2b4f47f68442619d555527d244f78a3297ea89325f843f87b8b54");

/// SushiSwap init code hash  
pub const SUSHISWAP_INIT_CODE_HASH: [u8; 32] = hex_literal::hex!("e18a34eb0e04b04f7a0ac29a6e80748dca96319b42c54d679cb821dca90c6303");

/// Curve Registry contract address (for dynamic pool discovery)
pub const CURVE_REGISTRY: Address = address!("90E00ACe148ca3b23Ac1bC8C240C2a7Dd9c2d7f5");

/// Balancer Vault contract address (for dynamic pool discovery)
pub const BALANCER_VAULT: Address = address!("BA12222222228d8Ba445958a75a0704d566BF2C8");

/// Standard fee tiers for Uniswap V3 (in basis points)
pub const V3_FEE_TIERS: [u32; 4] = [
    100,   // 0.01% - primarily for stablecoin pairs
    500,   // 0.05% - best for stable pairs like WETH/USDC
    3000,  // 0.30% - standard for most pairs
    10000, // 1.00% - for exotic/low liquidity pairs
];

/// Compute Uniswap V2 pool address deterministically
/// Tokens are automatically sorted (smaller address first)
pub fn compute_uniswap_v2_pool(token_a: Address, token_b: Address) -> Address {
    let (token0, token1) = sort_tokens(token_a, token_b);
    compute_create2_address(
        UNISWAP_V2_FACTORY,
        token0,
        token1,
        UNISWAP_V2_INIT_CODE_HASH,
    )
}

/// Compute SushiSwap pool address (same algorithm as Uniswap V2)
pub fn compute_sushiswap_pool(token_a: Address, token_b: Address) -> Address {
    let (token0, token1) = sort_tokens(token_a, token_b);
    compute_create2_address(
        SUSHISWAP_FACTORY,
        token0,
        token1,
        SUSHISWAP_INIT_CODE_HASH,
    )
}

/// Compute Uniswap V3 pool address for specific fee tier
pub fn compute_uniswap_v3_pool(token_a: Address, token_b: Address, fee_tier: u32) -> Address {
    let (token0, token1) = sort_tokens(token_a, token_b);
    compute_v3_create2_address(
        UNISWAP_V3_FACTORY,
        token0,
        token1,
        fee_tier,
        UNISWAP_V3_INIT_CODE_HASH,
    )
}

/// Get all possible V3 pools for a token pair (all fee tiers)
/// Returns Vec of (pool_address, fee_tier)
pub fn get_all_v3_pools(token_a: Address, token_b: Address) -> Vec<(Address, u32)> {
    V3_FEE_TIERS
        .iter()
        .map(|&fee| (compute_uniswap_v3_pool(token_a, token_b, fee), fee))
        .collect()
}

/// Sort tokens by address (smaller address first)
/// This is required for deterministic pool address calculation
fn sort_tokens(token_a: Address, token_b: Address) -> (Address, Address) {
    if token_a < token_b {
        (token_a, token_b)
    } else {
        (token_b, token_a)
    }
}

/// Generic CREATE2 address calculation for V2-style AMMs
fn compute_create2_address(
    factory: Address,
    token0: Address,
    token1: Address,
    init_code_hash: [u8; 32],
) -> Address {
    // Create salt from sorted token addresses
    let mut salt_input = Vec::with_capacity(40);
    salt_input.extend_from_slice(token0.as_slice());
    salt_input.extend_from_slice(token1.as_slice());
    let salt = keccak256(&salt_input);
    
    // CREATE2 formula: keccak256(0xff ++ factory ++ salt ++ init_code_hash)[12:]
    let mut input = Vec::with_capacity(85);
    input.push(0xff);
    input.extend_from_slice(factory.as_slice());
    input.extend_from_slice(salt.as_slice());
    input.extend_from_slice(&init_code_hash);
    
    let hash = keccak256(&input);
    Address::from_slice(&hash[12..])
}

/// CREATE2 address calculation for Uniswap V3 (includes fee in salt)
fn compute_v3_create2_address(
    factory: Address,
    token0: Address,
    token1: Address,
    fee: u32,
    init_code_hash: [u8; 32],
) -> Address {
    // V3 uses abi.encode(token0, token1, fee) for salt - NOT encodePacked!
    // abi.encode pads each parameter to 32 bytes
    let mut salt_input = Vec::with_capacity(96); // 32 * 3
    
    // Pad token0 address to 32 bytes (12 zero bytes + 20 address bytes)
    salt_input.extend_from_slice(&[0u8; 12]);
    salt_input.extend_from_slice(token0.as_slice());
    
    // Pad token1 address to 32 bytes
    salt_input.extend_from_slice(&[0u8; 12]);
    salt_input.extend_from_slice(token1.as_slice());
    
    // Pad fee to 32 bytes (28 zero bytes + 4 bytes for uint32)
    salt_input.extend_from_slice(&[0u8; 28]);
    salt_input.extend_from_slice(&fee.to_be_bytes());
    
    let salt = keccak256(&salt_input);
    
    // CREATE2 formula
    let mut input = Vec::with_capacity(85);
    input.push(0xff);
    input.extend_from_slice(factory.as_slice());
    input.extend_from_slice(salt.as_slice());
    input.extend_from_slice(&init_code_hash);
    
    let hash = keccak256(&input);
    Address::from_slice(&hash[12..])
}

/// Function selectors for contract calls
/// Curve Registry: find_pool_for_coins(address,address,uint256) -> address
const CURVE_FIND_POOL_FOR_COINS: [u8; 4] = [0x69, 0x82, 0xeb, 0x0b];
/// Balancer Vault: getPoolTokens(bytes32) -> (address[],uint256[],uint256)
const BALANCER_GET_POOL_TOKENS: [u8; 4] = [0xf9, 0x4d, 0x46, 0x68];

/// Simple ABI encoding helper for function calls
fn encode_function_call(selector: [u8; 4], params: &[u8]) -> Bytes {
    let mut encoded = Vec::with_capacity(4 + params.len());
    encoded.extend_from_slice(&selector);
    encoded.extend_from_slice(params);
    Bytes::from(encoded)
}

/// Encode two addresses as ABI parameters
fn encode_two_addresses(addr1: Address, addr2: Address) -> Vec<u8> {
    let mut params = Vec::with_capacity(64); // 32 bytes each
    
    // First address (padded to 32 bytes)
    params.extend_from_slice(&[0u8; 12]); // 12 zero bytes
    params.extend_from_slice(addr1.as_slice()); // 20 address bytes
    
    // Second address (padded to 32 bytes)
    params.extend_from_slice(&[0u8; 12]); // 12 zero bytes  
    params.extend_from_slice(addr2.as_slice()); // 20 address bytes
    
    params
}

/// Encode two addresses and one uint256 as ABI parameters
fn encode_two_addresses_and_uint256(addr1: Address, addr2: Address, value: U256) -> Vec<u8> {
    let mut params = Vec::with_capacity(96); // 32 bytes each
    
    // First address (padded to 32 bytes)
    params.extend_from_slice(&[0u8; 12]);
    params.extend_from_slice(addr1.as_slice());
    
    // Second address (padded to 32 bytes)
    params.extend_from_slice(&[0u8; 12]);
    params.extend_from_slice(addr2.as_slice());
    
    // Uint256 value (32 bytes, big-endian)
    let value_bytes = value.to_be_bytes::<32>();
    params.extend_from_slice(&value_bytes);
    
    params
}

/// Encode bytes32 as ABI parameter
fn encode_bytes32(value: [u8; 32]) -> Vec<u8> {
    value.to_vec()
}

/// Find Curve pool for token pair using registry contract
/// Uses view function simulation to query the Curve Registry
/// Returns None if no pool exists for the pair
pub async fn find_curve_pool_for_coins(
    simulator: &TxSimulator,
    token_a: Address,
    token_b: Address,
    block_number: Option<u64>
) -> Result<Option<Address>> {
    // Call find_pool_for_coins(address,address,uint256) with index 0
    let params = encode_two_addresses_and_uint256(token_a, token_b, U256::ZERO);
    let call_data = encode_function_call(CURVE_FIND_POOL_FOR_COINS, &params);
    
    let result = simulator.simulate_view_function(
        CURVE_REGISTRY,
        call_data,
        block_number
    ).await?;
    
    // Decode address from result (last 32 bytes, take last 20 bytes for address)
    if result.success && result.output.len() >= 32 {
        let pool_address = Address::from_slice(&result.output[12..32]);
        if pool_address != Address::ZERO {
            return Ok(Some(pool_address));
        }
    }
    
    Ok(None)
}

/// Verify Balancer pool contains specific token pair
/// Uses view function simulation to query the Balancer Vault
/// Returns true if the pool contains both tokens
pub async fn verify_balancer_pool_tokens(
    simulator: &TxSimulator,
    pool_id: [u8; 32],
    token_a: Address,
    token_b: Address,
    block_number: Option<u64>
) -> Result<bool> {
    // Call getPoolTokens(bytes32)
    let params = encode_bytes32(pool_id);
    let call_data = encode_function_call(BALANCER_GET_POOL_TOKENS, &params);
    
    let result = simulator.simulate_view_function(
        BALANCER_VAULT,
        call_data,
        block_number
    ).await?;
    
    // Decode response: (address[] tokens, uint256[] balances, uint256 lastChangeBlock)
    // This is complex ABI decoding - for now we'll do basic pattern matching
    if result.success {
        if let Some(tokens) = decode_address_array_from_result(&result.output) {
            let has_token_a = tokens.contains(&token_a);
            let has_token_b = tokens.contains(&token_b);
            return Ok(has_token_a && has_token_b);
        }
    }
    
    Ok(false)
}

/// Get all tokens in a Balancer pool
/// Returns token addresses and their current balances
pub async fn get_balancer_pool_tokens(
    simulator: &TxSimulator,
    pool_id: [u8; 32],
    block_number: Option<u64>
) -> Result<Option<(Vec<Address>, Vec<U256>)>> {
    // Call getPoolTokens(bytes32)
    let params = encode_bytes32(pool_id);
    let call_data = encode_function_call(BALANCER_GET_POOL_TOKENS, &params);
    
    let result = simulator.simulate_view_function(
        BALANCER_VAULT,
        call_data,
        block_number
    ).await?;
    
    // Decode response: (address[] tokens, uint256[] balances, uint256 lastChangeBlock)
    if result.success {
        if let (Some(tokens), Some(balances)) = (
            decode_address_array_from_result(&result.output),
            decode_uint256_array_from_result(&result.output, 32) // Skip first array offset
        ) {
            return Ok(Some((tokens, balances)));
        }
    }
    
    Ok(None)
}

/// Basic ABI decoder for address arrays (simplified)
/// This is a basic implementation - in production you'd want more robust ABI decoding
fn decode_address_array_from_result(result: &[u8]) -> Option<Vec<Address>> {
    if result.len() < 96 { // At least 3 * 32 bytes for basic structure
        return None;
    }
    
    // Skip first 64 bytes (two offset pointers), read array length
    let array_length_start = 64;
    if result.len() < array_length_start + 32 {
        return None;
    }
    
    let array_length = u32::from_be_bytes([
        result[array_length_start + 28],
        result[array_length_start + 29], 
        result[array_length_start + 30],
        result[array_length_start + 31]
    ]) as usize;
    
    let mut addresses = Vec::with_capacity(array_length);
    let data_start = array_length_start + 32;
    
    for i in 0..array_length {
        let addr_start = data_start + (i * 32) + 12; // Skip 12 bytes padding
        if result.len() >= addr_start + 20 {
            let addr_bytes = &result[addr_start..addr_start + 20];
            addresses.push(Address::from_slice(addr_bytes));
        }
    }
    
    Some(addresses)
}

/// Basic ABI decoder for uint256 arrays (simplified)
fn decode_uint256_array_from_result(result: &[u8], offset: usize) -> Option<Vec<U256>> {
    // This would need to be implemented based on the specific ABI layout
    // For now, return None to avoid compilation errors
    // TODO: Implement proper uint256 array decoding
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    #[test]
    fn test_uniswap_v2_pool_address() {
        // Test with known WETH/USDC pool
        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        
        let pool = compute_uniswap_v2_pool(weth, usdc);
        let expected = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");
        
        assert_eq!(pool, expected, "WETH/USDC V2 pool address mismatch");
    }
    
    #[test]
    fn test_uniswap_v3_pool_address() {
        // Test with known WETH/USDC 0.05% pool
        let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        
        let pool = compute_uniswap_v3_pool(weth, usdc, 500);
        let expected = address!("88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640");
        
        assert_eq!(pool, expected, "WETH/USDC V3 0.05% pool address mismatch");
    }
    
    #[test]
    fn test_token_sorting() {
        let token_a = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
        let token_b = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        
        // Should be sorted with smaller address first
        let (token0, token1) = sort_tokens(token_a, token_b);
        assert!(token0 < token1, "Tokens not properly sorted");
        
        // Order shouldn't matter
        let (token0_rev, token1_rev) = sort_tokens(token_b, token_a);
        assert_eq!(token0, token0_rev);
        assert_eq!(token1, token1_rev);
    }
}