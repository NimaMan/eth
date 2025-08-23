/// Pool adapter traits and implementations

pub mod uniswap_v2;
pub mod uniswap_v3;

use alloy_primitives::{Address, U256};
use eyre::Result;
use reth_tx_simulator::CallRequest;

pub use uniswap_v2::UniswapV2Adapter;
pub use uniswap_v3::UniswapV3Adapter;

/// Trait for pool-specific transaction building
pub trait PoolAdapter: Send + Sync {
    /// Build a buy transaction (ETH -> Token)
    fn build_buy_transaction(
        &self,
        token_address: Address,
        eth_amount: U256,
        buyer_address: Address,
        slippage: f64,
    ) -> Result<CallRequest>;
    
    /// Build an approve transaction for the router
    fn build_approve_transaction(
        &self,
        token_address: Address,
        amount: U256,
        buyer_address: Address,
    ) -> Result<CallRequest>;
    
    /// Build a sell transaction (Token -> ETH)
    fn build_sell_transaction(
        &self,
        token_address: Address,
        token_amount: U256,
        buyer_address: Address,
        slippage: f64,
    ) -> Result<CallRequest>;
    
    /// Get the router address for this pool type
    fn router_address(&self) -> Address;
    
    /// Get WETH address
    fn weth_address(&self) -> Address {
        // Mainnet WETH
        Address::from([
            0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 
            0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 
            0x3C, 0x75, 0x6C, 0xc2
        ])
    }
}