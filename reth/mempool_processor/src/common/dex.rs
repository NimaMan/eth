// DEX interaction detection utilities
//
// This module provides utilities for detecting interactions with decentralized exchanges (DEXs)
// like Uniswap V2, V3, and V4.

use ethers::abi::{decode, ParamType, Token};
use ethers::types::{Address, U256};
use hex;
use tracing::{debug, trace};

/// Common Uniswap V2 Router addresses
pub const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";

/// Common Uniswap V3 Router addresses
pub const UNISWAP_V3_ROUTER: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
pub const UNISWAP_V3_ROUTER_2: &str = "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45";

/// Common DEX Router method signatures
pub mod router_method_ids {
    // Uniswap V2 Router
    pub const SWAP_EXACT_TOKENS_FOR_ETH: &[u8] = &[0x18, 0xcb, 0xaf, 0xe5]; // swapExactTokensForETH
    pub const SWAP_EXACT_ETH_FOR_TOKENS: &[u8] = &[0x7f, 0xf3, 0x6a, 0xb5]; // swapExactETHForTokens
    pub const SWAP_TOKENS_FOR_EXACT_ETH: &[u8] = &[0x4a, 0x25, 0xd9, 0x4a]; // swapTokensForExactETH
    pub const SWAP_ETH_FOR_EXACT_TOKENS: &[u8] = &[0xfb, 0x3b, 0xdb, 0x41]; // swapETHForExactTokens
    
    // Uniswap V3 Router
    pub const EXACT_INPUT_SINGLE: &[u8] = &[0x41, 0x4b, 0xf3, 0x89]; // exactInputSingle
    pub const EXACT_OUTPUT_SINGLE: &[u8] = &[0xdb, 0x3e, 0x21, 0x98]; // exactOutputSingle
    pub const MULTICALL: &[u8] = &[0xac, 0x96, 0x50, 0xd8]; // multicall (V3 often uses this)
}

/// Pool method signatures (direct pool interactions)
pub mod pool_method_ids {
    pub const SWAP: &[u8] = &[0x02, 0x8f, 0x6c, 0x0e]; // swap (Uniswap V2/V3 pools)
    pub const MINT: &[u8] = &[0x6a, 0x62, 0x78, 0x42]; // mint (add liquidity)
    pub const BURN: &[u8] = &[0x89, 0xaf, 0xcb, 0x44]; // burn (remove liquidity)
    pub const SYNC: &[u8] = &[0xff, 0xf6, 0xca, 0xe9]; // sync
}

/// Represents a detected DEX interaction
#[derive(Debug, Clone)]
pub enum DexInteraction {
    /// Token swap via router
    RouterSwap {
        router_address: Address,
        swap_type: SwapType,
        token_address: Option<Address>,
        eth_amount: Option<U256>,
        token_amount: Option<U256>,
    },
    /// Direct pool interaction
    PoolInteraction {
        pool_address: Address,
        interaction_type: PoolInteractionType,
    },
    /// Unknown DEX interaction
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SwapType {
    TokensForETH,
    ETHForTokens,
    TokensForTokens,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PoolInteractionType {
    Swap,
    AddLiquidity,
    RemoveLiquidity,
    Sync,
}

/// Check if an address is a known DEX router
pub fn is_dex_router(address: &str) -> bool {
    let addr_lower = address.to_lowercase();
    addr_lower == UNISWAP_V2_ROUTER.to_lowercase() ||
    addr_lower == UNISWAP_V3_ROUTER.to_lowercase() ||
    addr_lower == UNISWAP_V3_ROUTER_2.to_lowercase()
}

/// Detect DEX interactions from transaction data
pub fn detect_dex_interaction(to_address: &str, input_data: &[u8]) -> Option<DexInteraction> {
    if input_data.len() < 4 {
        return None;
    }
    
    let method_id = &input_data[0..4];
    
    // Check if it's a router interaction
    if is_dex_router(to_address) {
        return detect_router_interaction(to_address, method_id, &input_data[4..]);
    }
    
    // Check for direct pool interactions
    detect_pool_interaction(to_address, method_id)
}

/// Detect router-based DEX interactions
fn detect_router_interaction(router_address: &str, method_id: &[u8], params: &[u8]) -> Option<DexInteraction> {
    let router_addr = router_address.parse::<Address>().ok()?;
    
    match method_id {
        router_method_ids::SWAP_EXACT_TOKENS_FOR_ETH => {
            debug!("Detected swapExactTokensForETH on router {}", router_address);
            Some(DexInteraction::RouterSwap {
                router_address: router_addr,
                swap_type: SwapType::TokensForETH,
                token_address: None, // Would need to decode path
                eth_amount: None,
                token_amount: None,
            })
        },
        router_method_ids::SWAP_EXACT_ETH_FOR_TOKENS => {
            debug!("Detected swapExactETHForTokens on router {}", router_address);
            Some(DexInteraction::RouterSwap {
                router_address: router_addr,
                swap_type: SwapType::ETHForTokens,
                token_address: None, // Would need to decode path
                eth_amount: None,
                token_amount: None,
            })
        },
        router_method_ids::EXACT_INPUT_SINGLE => {
            debug!("Detected V3 exactInputSingle on router {}", router_address);
            // V3 swaps are more complex, would need struct decoding
            Some(DexInteraction::RouterSwap {
                router_address: router_addr,
                swap_type: SwapType::TokensForTokens,
                token_address: None,
                eth_amount: None,
                token_amount: None,
            })
        },
        router_method_ids::MULTICALL => {
            debug!("Detected multicall on router {} (likely V3 swap)", router_address);
            Some(DexInteraction::RouterSwap {
                router_address: router_addr,
                swap_type: SwapType::TokensForTokens,
                token_address: None,
                eth_amount: None,
                token_amount: None,
            })
        },
        _ => None,
    }
}

/// Detect direct pool interactions
fn detect_pool_interaction(pool_address: &str, method_id: &[u8]) -> Option<DexInteraction> {
    let pool_addr = pool_address.parse::<Address>().ok()?;
    
    let interaction_type = match method_id {
        pool_method_ids::SWAP => {
            debug!("Detected direct pool swap on {}", pool_address);
            PoolInteractionType::Swap
        },
        pool_method_ids::MINT => {
            debug!("Detected liquidity addition on pool {}", pool_address);
            PoolInteractionType::AddLiquidity
        },
        pool_method_ids::BURN => {
            debug!("Detected liquidity removal on pool {}", pool_address);
            PoolInteractionType::RemoveLiquidity
        },
        pool_method_ids::SYNC => {
            trace!("Detected pool sync on {}", pool_address);
            PoolInteractionType::Sync
        },
        _ => return None,
    };
    
    Some(DexInteraction::PoolInteraction {
        pool_address: pool_addr,
        interaction_type,
    })
}

/// Check if a transaction might affect a token based on router paths
/// This is a simplified version - full implementation would decode the path parameter
pub fn might_affect_token(dex_interaction: &DexInteraction, token_address: &str) -> bool {
    match dex_interaction {
        DexInteraction::RouterSwap { .. } => {
            // In a full implementation, we would decode the path parameter
            // to see if it includes the token address
            true // For now, assume any router swap might affect the token
        },
        DexInteraction::PoolInteraction { .. } => {
            // Would need to check if the pool contains the token
            true
        },
        DexInteraction::Unknown => false,
    }
}