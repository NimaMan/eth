/// Type definitions for trading viability analysis

use alloy_primitives::{Address, U256};
use crate::data_models::ProcessedTransaction;
use serde::{Serialize, Deserialize};

/// Supported DEX pool types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolType {
    UniswapV2,
    UniswapV3 { fee_tier: u32 }, // 500, 3000, 10000 (0.05%, 0.3%, 1%)
    SushiSwap,
    Curve,
    Balancer,
}

impl PoolType {
    /// Get the router address for this pool type
    pub fn router_address(&self) -> Address {
        match self {
            PoolType::UniswapV2 | PoolType::SushiSwap => {
                // Uniswap V2 Router 02
                Address::from([
                    0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53, 
                    0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc, 0xb4, 0xc6, 
                    0x59, 0xF2, 0x48, 0x8D
                ])
            },
            PoolType::UniswapV3 { .. } => {
                // Uniswap V3 SwapRouter
                Address::from([
                    0xE5, 0x92, 0x42, 0x7A, 0x0A, 0xEc, 0xe9, 0x2D,
                    0xe3, 0xEd, 0xee, 0x1F, 0x18, 0xE0, 0x15, 0x7C,
                    0x05, 0x86, 0x15, 0x64
                ])
            },
            PoolType::Curve => {
                // Curve Registry Exchange
                Address::from([
                    0x81, 0xC4, 0x6F, 0xDC, 0x50, 0x30, 0x56, 0x5b,
                    0xBC, 0x29, 0x3c, 0x68, 0x11, 0x48, 0x65, 0xf9,
                    0x86, 0x48, 0x03, 0x04
                ])
            },
            PoolType::Balancer => {
                // Balancer Vault
                Address::from([
                    0xBA, 0x12, 0x22, 0x22, 0x22, 0x28, 0xD8, 0x4A,
                    0x5C, 0xFC, 0xDD, 0x74, 0x26, 0x6c, 0x93, 0x0E,
                    0x38, 0xfd, 0x7D, 0xf6
                ])
            },
        }
    }
}

/// Result of pool viability analysis
#[derive(Debug, Clone)]
pub struct PoolViabilityResult {
    /// The type of pool analyzed
    pub pool_type: PoolType,
    
    /// Pool contract address
    pub pool_address: Address,
    
    /// Token contract address
    pub token_address: Address,
    
    /// Whether trading is possible (buy and sell both succeed)
    pub is_tradeable: bool,
    
    /// Buy tax percentage (negative means failed)
    pub buy_tax_percent: f64,
    
    /// Sell tax percentage (negative means failed)
    pub sell_tax_percent: f64,
    
    /// Amount of tokens received from buy
    pub tokens_received: U256,
    
    /// Amount of ETH spent in buy
    pub eth_spent: U256,
    
    /// Amount of ETH received from sell
    pub eth_received: U256,
    
    /// Processed buy transaction
    pub buy_transaction: ProcessedTransaction,
    
    /// Processed sell transaction
    pub sell_transaction: ProcessedTransaction,
    
    /// Processed approve transaction
    pub approve_transaction: ProcessedTransaction,
    
    /// Optional prior transaction result
    pub prior_transaction: Option<ProcessedTransaction>,
    
    /// Failure reason if not tradeable
    pub failure_reason: Option<String>,
    
    /// Block number used for simulation
    pub block_number: u64,
}