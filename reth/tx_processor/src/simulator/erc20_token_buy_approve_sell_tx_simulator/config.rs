/// Configuration for pool viability analysis

use alloy_primitives::{Address, U256};
use crate::tx_processor::data_models::ProcessedTransaction;
use super::types::PoolType;

/// Configuration for analyzing pool viability
#[derive(Debug, Clone)]
pub struct PoolViabilityConfig {
    /// Token contract address
    pub token_address: Address,
    
    /// Pool contract address
    pub pool_address: Address,
    
    /// Type of pool (V2, V3, etc.)
    pub pool_type: PoolType,
    
    /// Amount of ETH to test with (default: 0.01 ETH)
    pub test_amount: U256,
    
    /// Address to use as buyer in simulation
    pub buyer_address: Address,
    
    /// Optional prior transaction (e.g., enable trading)
    pub prior_tx: Option<ProcessedTransaction>,
    
    /// Block number to simulate at (None = latest)
    pub block_number: Option<u64>,
    
    /// Slippage tolerance (default: 0.5%)
    pub slippage_tolerance: f64,
    
    /// Gas limit for transactions
    pub gas_limit: u64,
    
    /// Gas price (fallback)
    pub gas_price: u128,
    
    /// WETH address
    pub weth_address: Address,
    
    /// Block delay between buy/approve and sell (default: 0 = same block)
    pub block_delay: u64,
    
    /// Token decimals (e.g., 18 for ETH, 6 for USDC, 9 for FLOKI)
    pub token_decimals: u8,
}

impl Default for PoolViabilityConfig {
    fn default() -> Self {
        Self {
            // These will be set by the user
            token_address: Address::ZERO,
            pool_address: Address::ZERO,
            pool_type: PoolType::UniswapV2,
            
            // Default test with 0.01 ETH
            test_amount: U256::from(10_000_000_000_000_000u64),
            
            // Default test buyer address
            buyer_address: Address::from([
                0x0C, 0x96, 0xc6, 0x02, 0xb1, 0xb3, 0x32, 0xB8, 
                0xAB, 0x20, 0x93, 0xE5, 0xd7, 0x2D, 0x80, 0x4a, 
                0x24, 0xbd, 0x56, 0x89
            ]),
            
            prior_tx: None,
            block_number: None,
            slippage_tolerance: 0.5,
            gas_limit: 300_000,
            gas_price: 100_000_000_000, // 100 gwei
            
            // Mainnet WETH
            weth_address: Address::from([
                0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 
                0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 
                0x3C, 0x75, 0x6C, 0xc2
            ]),
            
            // Default: same block execution
            block_delay: 0,
            
            // Default: 18 decimals (ETH standard)
            token_decimals: 18,
        }
    }
}

impl PoolViabilityConfig {
    /// Create a new config with required fields
    pub fn new(
        token_address: Address,
        pool_address: Address,
        pool_type: PoolType,
    ) -> Self {
        Self {
            token_address,
            pool_address,
            pool_type,
            ..Default::default()
        }
    }
    
    /// Set test amount
    pub fn with_test_amount(mut self, amount: U256) -> Self {
        self.test_amount = amount;
        self
    }
    
    /// Set buyer address
    pub fn with_buyer(mut self, address: Address) -> Self {
        self.buyer_address = address;
        self
    }
    
    /// Set prior transaction
    pub fn with_prior_tx(mut self, tx: ProcessedTransaction) -> Self {
        self.prior_tx = Some(tx);
        self
    }
    
    /// Set block number
    pub fn with_block(mut self, block: u64) -> Self {
        self.block_number = Some(block);
        self
    }
    
    /// Set block delay between buy/approve and sell
    pub fn with_block_delay(mut self, delay: u64) -> Self {
        self.block_delay = delay;
        self
    }
    
    /// Set token decimals
    pub fn with_token_decimals(mut self, decimals: u8) -> Self {
        self.token_decimals = decimals;
        self
    }
}