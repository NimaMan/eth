/// Configuration for buy/sell simulation
/// 
/// Default values are taken from mempool_processor for consistency

use alloy_primitives::{Address, U256};

/// Configuration for buy/sell simulation
#[derive(Debug, Clone)]
pub struct BuySellConfig {
    /// Amount of ETH to use for test buy (default: 0.01 ETH)
    pub test_buy_amount: U256,
    /// Router address for swaps (default: Uniswap V2)
    pub router_address: Address,
    /// WETH address (default: mainnet WETH)
    pub weth_address: Address,
    /// Gas limit for transactions
    pub gas_limit: u64,
    /// Gas price in wei (fallback only)
    pub gas_price: u128,
    /// Buyer address for simulations
    pub buyer_address: Address,
    /// Deadline for swaps (seconds from now)
    pub deadline_seconds: u64,
}

impl Default for BuySellConfig {
    fn default() -> Self {
        Self {
            // 0.01 ETH (same as mempool_processor)
            test_buy_amount: U256::from(10_000_000_000_000_000u64),
            
            // Uniswap V2 Router
            router_address: Address::from([
                0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53, 
                0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc, 0xb4, 0xc6, 
                0x59, 0xF2, 0x48, 0x8D
            ]),
            
            // WETH
            weth_address: Address::from([
                0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 
                0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 
                0x3C, 0x75, 0x6C, 0xc2
            ]),
            
            gas_limit: 300_000,
            gas_price: 100_000_000_000u128, // 100 gwei fallback
            
            // Fixed test address (same as mempool_processor)
            buyer_address: Address::from([
                0x0C, 0x96, 0xc6, 0x02, 0xb1, 0xb3, 0x32, 0xB8, 
                0xAB, 0x20, 0x93, 0xE5, 0xd7, 0x2D, 0x80, 0x4a, 
                0x24, 0xbd, 0x56, 0x89
            ]),
            
            deadline_seconds: 300, // 5 minutes
        }
    }
}

impl BuySellConfig {
    /// Create config with custom buy amount
    pub fn with_buy_amount(mut self, amount: U256) -> Self {
        self.test_buy_amount = amount;
        self
    }
    
    /// Create config with custom buyer address
    pub fn with_buyer(mut self, address: Address) -> Self {
        self.buyer_address = address;
        self
    }
    
    /// Create config with custom router
    pub fn with_router(mut self, address: Address) -> Self {
        self.router_address = address;
        self
    }
}