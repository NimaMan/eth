/// Configuration for pool viability analysis

use alloy_primitives::{Address, U256};
use crate::tx_processor::data_models::ProcessedTransaction;
use super::types::PoolType;

#[derive(Debug, Clone)]
pub struct PoolViabilityConfig {
    pub token_address: Address,
    pub pool_address: Address,
    pub pool_type: PoolType,
    pub test_amount: U256,
    pub buyer_address: Address,
    pub prior_tx: Option<ProcessedTransaction>,
    pub block_number: Option<u64>,
    pub slippage_tolerance: f64,
    pub gas_limit: u64,
    pub gas_price: u128,
    pub weth_address: Address,
    pub block_delay: u64,
    pub token_decimals: u8,
}

impl Default for PoolViabilityConfig {
    fn default() -> Self {
        Self {
            token_address: Address::ZERO,
            pool_address: Address::ZERO,
            pool_type: PoolType::UniswapV2,
            test_amount: U256::from(10_000_000_000_000_000u64),
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
            weth_address: Address::from([
                0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 
                0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xd9, 0x08, 
                0x3C, 0x75, 0x6C, 0xc2
            ]),
            block_delay: 0,
            token_decimals: 18,
        }
    }
}

impl PoolViabilityConfig {
    pub fn new(token_address: Address, pool_address: Address, pool_type: PoolType) -> Self {
        Self { token_address, pool_address, pool_type, ..Default::default() }
    }
    pub fn with_test_amount(mut self, amount: U256) -> Self { self.test_amount = amount; self }
    pub fn with_buyer(mut self, address: Address) -> Self { self.buyer_address = address; self }
    pub fn with_prior_tx(mut self, tx: ProcessedTransaction) -> Self { self.prior_tx = Some(tx); self }
    pub fn with_block(mut self, block: u64) -> Self { self.block_number = Some(block); self }
    pub fn with_block_delay(mut self, delay: u64) -> Self { self.block_delay = delay; self }
    pub fn with_token_decimals(mut self, decimals: u8) -> Self { self.token_decimals = decimals; self }
}

