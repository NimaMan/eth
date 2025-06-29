//! Uniswap V2 protocol implementation
//! 
//! Implements the DexProtocol and DexPool traits for Uniswap V2 pools,
//! providing optimized swap execution with pre-computed selectors.

mod pool;
mod protocol;

pub use pool::UniswapV2Pool;
pub use protocol::UniswapV2Protocol;

use lazy_static::lazy_static;
use ethers::prelude::*;

/// Uniswap V2 contract addresses
pub mod addresses {
    use super::*;
    
    lazy_static! {
        /// Uniswap V2 Factory address on mainnet
        pub static ref FACTORY: Address = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".parse().unwrap();
        
        /// Uniswap V2 Router address on mainnet
        pub static ref ROUTER: Address = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse().unwrap();
        
        /// WETH address on mainnet
        pub static ref WETH: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap();
    }
}

/// Pre-computed function selectors for optimal performance
pub mod selectors {
    use lazy_static::lazy_static;
    
    lazy_static! {
        /// swapExactETHForTokens(uint256,address[],address,uint256)
        pub static ref SWAP_ETH_FOR_TOKENS: [u8; 4] = {
            let hash = ethers::utils::keccak256("swapExactETHForTokens(uint256,address[],address,uint256)");
            [hash[0], hash[1], hash[2], hash[3]]
        };
        
        /// swapExactTokensForETH(uint256,uint256,address[],address,uint256)
        pub static ref SWAP_TOKENS_FOR_ETH: [u8; 4] = {
            let hash = ethers::utils::keccak256("swapExactTokensForETH(uint256,uint256,address[],address,uint256)");
            [hash[0], hash[1], hash[2], hash[3]]
        };
        
        /// swapExactTokensForTokens(uint256,uint256,address[],address,uint256)
        pub static ref SWAP_TOKENS_FOR_TOKENS: [u8; 4] = {
            let hash = ethers::utils::keccak256("swapExactTokensForTokens(uint256,uint256,address[],address,uint256)");
            [hash[0], hash[1], hash[2], hash[3]]
        };
        
        /// getReserves()
        pub static ref GET_RESERVES: [u8; 4] = {
            let hash = ethers::utils::keccak256("getReserves()");
            [hash[0], hash[1], hash[2], hash[3]]
        };
        
        /// token0()
        pub static ref TOKEN0: [u8; 4] = {
            let hash = ethers::utils::keccak256("token0()");
            [hash[0], hash[1], hash[2], hash[3]]
        };
        
        /// token1()
        pub static ref TOKEN1: [u8; 4] = {
            let hash = ethers::utils::keccak256("token1()");
            [hash[0], hash[1], hash[2], hash[3]]
        };
    }
}

/// Uniswap V2 constants
pub mod constants {
    /// Fee for all Uniswap V2 pools (0.3%)
    pub const FEE_BPS: u32 = 30;
    
    /// Fee denominator
    pub const FEE_DENOMINATOR: u32 = 10_000;
    
    /// Minimum liquidity locked
    pub const MINIMUM_LIQUIDITY: u128 = 1000;
}

/// Calculate Uniswap V2 pair address using CREATE2
pub fn calculate_pair_address(token_a: Address, token_b: Address) -> Address {
    use ethers::abi::{encode, Token};
    
    let (token0, token1) = if token_a < token_b {
        (token_a, token_b)
    } else {
        (token_b, token_a)
    };
    
    // Uniswap V2 init code hash
    let init_code_hash = hex::decode("96e8ac4277198ff8b6f785478aa9a39f403cb768dd02cbee326c3e7da348845f")
        .expect("Valid init code hash");
    
    // Compute CREATE2 address
    let salt = ethers::utils::keccak256(encode(&[
        Token::Address(token0),
        Token::Address(token1),
    ]));
    
    let mut data = Vec::with_capacity(85);
    data.push(0xff);
    data.extend_from_slice(addresses::FACTORY.as_bytes());
    data.extend_from_slice(&salt);
    data.extend_from_slice(&init_code_hash);
    
    let hash = ethers::utils::keccak256(&data);
    Address::from_slice(&hash[12..])
}

/// Calculate output amount using constant product formula
pub fn calculate_amount_out(
    amount_in: U256,
    reserve_in: U256,
    reserve_out: U256,
) -> U256 {
    if amount_in.is_zero() || reserve_in.is_zero() || reserve_out.is_zero() {
        return U256::zero();
    }
    
    let amount_in_with_fee = amount_in * 997; // 0.3% fee
    let numerator = amount_in_with_fee * reserve_out;
    let denominator = (reserve_in * 1000) + amount_in_with_fee;
    
    numerator / denominator
}