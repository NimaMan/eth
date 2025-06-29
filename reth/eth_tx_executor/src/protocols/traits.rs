//! Core trait definitions for DEX protocol abstractions
//! 
//! These traits define the common interface that all DEX protocols must implement,
//! allowing the system to work with different protocols interchangeably.

use crate::common::{Result, TokenAddress, TokenAmount, PoolAddress, GasPrice};
use async_trait::async_trait;
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::fmt::Debug;

/// Information about a liquidity pool
#[derive(Debug, Clone)]
pub struct PoolInfo {
    /// Pool address
    pub address: PoolAddress,
    /// Protocol name (e.g., "UniswapV2", "UniswapV3")
    pub protocol: String,
    /// Token A address
    pub token_a: TokenAddress,
    /// Token B address
    pub token_b: TokenAddress,
    /// Fee tier (basis points, e.g., 30 = 0.3%)
    pub fee_bps: u32,
    /// Current reserves (token_a, token_b)
    pub reserves: Option<(U256, U256)>,
    /// Pool TVL in USD (if available)
    pub tvl_usd: Option<f64>,
}

/// Quote for a potential swap
#[derive(Debug, Clone)]
pub struct SwapQuote {
    /// Amount of tokens out
    pub amount_out: TokenAmount,
    /// Minimum amount out (with slippage)
    pub amount_out_min: TokenAmount,
    /// Price impact percentage
    pub price_impact: f64,
    /// Gas estimate for the swap
    pub gas_estimate: U256,
    /// Suggested gas price
    pub gas_price: GasPrice,
    /// Route taken for the swap
    pub route: SwapRoute,
    /// Additional protocol-specific data
    pub metadata: Option<serde_json::Value>,
}

/// Route information for a swap
#[derive(Debug, Clone)]
pub struct SwapRoute {
    /// Path of tokens for the swap
    pub path: Vec<TokenAddress>,
    /// Pools used in the route
    pub pools: Vec<PoolAddress>,
    /// Fee tiers for each hop (if applicable)
    pub fees: Vec<u32>,
}

/// Core trait for interacting with a DEX pool
#[async_trait]
pub trait DexPool: Send + Sync + Debug {
    /// Get pool information
    fn info(&self) -> &PoolInfo;
    
    /// Get current reserves
    async fn get_reserves(&self) -> Result<(U256, U256)>;
    
    /// Get a quote for swapping tokens
    async fn get_swap_quote(
        &self,
        token_in: TokenAddress,
        token_out: TokenAddress,
        amount_in: TokenAmount,
    ) -> Result<SwapQuote>;
    
    /// Build a swap transaction
    async fn build_swap_transaction(
        &self,
        token_in: TokenAddress,
        token_out: TokenAddress,
        amount_in: TokenAmount,
        amount_out_min: TokenAmount,
        recipient: Address,
        deadline: U256,
    ) -> Result<TypedTransaction>;
    
    /// Estimate gas for a swap
    async fn estimate_swap_gas(
        &self,
        token_in: TokenAddress,
        token_out: TokenAddress,
        amount_in: TokenAmount,
    ) -> Result<U256>;
    
    /// Check if the pool supports a token pair
    fn supports_pair(&self, token_a: TokenAddress, token_b: TokenAddress) -> bool {
        (self.info().token_a == token_a && self.info().token_b == token_b) ||
        (self.info().token_a == token_b && self.info().token_b == token_a)
    }
    
    /// Get the protocol-specific router address
    fn router_address(&self) -> Address;
    
    /// Clone the pool as a trait object
    fn clone_box(&self) -> Box<dyn DexPool>;
}

/// Trait for liquidity-specific operations
#[async_trait]
pub trait LiquidityPool: DexPool {
    /// Add liquidity to the pool
    async fn build_add_liquidity_transaction(
        &self,
        token_a_amount: TokenAmount,
        token_b_amount: TokenAmount,
        token_a_min: TokenAmount,
        token_b_min: TokenAmount,
        recipient: Address,
        deadline: U256,
    ) -> Result<TypedTransaction>;
    
    /// Remove liquidity from the pool
    async fn build_remove_liquidity_transaction(
        &self,
        liquidity_amount: U256,
        token_a_min: TokenAmount,
        token_b_min: TokenAmount,
        recipient: Address,
        deadline: U256,
    ) -> Result<TypedTransaction>;
    
    /// Get LP token balance for an address
    async fn get_lp_balance(&self, address: Address) -> Result<U256>;
}

/// Factory trait for creating protocol-specific pools
#[async_trait]
pub trait ProtocolFactory: Send + Sync {
    /// Create a pool instance from an address
    async fn create_pool(&self, address: PoolAddress) -> Result<Box<dyn DexPool>>;
    
    /// Find pool address for a token pair
    async fn find_pool_address(
        &self,
        token_a: TokenAddress,
        token_b: TokenAddress,
    ) -> Result<Option<PoolAddress>>;
    
    /// Get all pools for a token
    async fn get_pools_for_token(&self, token: TokenAddress) -> Result<Vec<PoolAddress>>;
}

/// Main trait for DEX protocol implementations
#[async_trait]
pub trait DexProtocol: Send + Sync {
    /// Get the protocol name
    fn name(&self) -> &'static str;
    
    /// Get the protocol version
    fn version(&self) -> &'static str;
    
    /// Get the factory contract address
    fn factory_address(&self) -> Address;
    
    /// Get the router contract address
    fn router_address(&self) -> Address;
    
    /// Find a pool for a token pair
    async fn find_pool(
        &self,
        token_a: TokenAddress,
        token_b: TokenAddress,
    ) -> Result<Option<Box<dyn DexPool>>>;
    
    /// Get all active pools (limited to prevent overwhelming the system)
    async fn get_active_pools(&self, limit: usize) -> Result<Vec<Box<dyn DexPool>>>;
    
    /// Check if the protocol supports a specific feature
    fn supports_feature(&self, feature: ProtocolFeature) -> bool;
}

/// Features that protocols may or may not support
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolFeature {
    /// Multiple fee tiers for the same pair
    MultipleFees,
    /// Concentrated liquidity
    ConcentratedLiquidity,
    /// Flash loans
    FlashLoans,
    /// Native ETH support (vs WETH only)
    NativeEth,
    /// Dynamic fees
    DynamicFees,
    /// Limit orders
    LimitOrders,
}

/// Helper trait for cloning trait objects
impl Clone for Box<dyn DexPool> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}