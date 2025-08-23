/// Trading Viability Analysis Module
/// 
/// Analyzes if a token pool is tradeable and calculates associated taxes/fees.
/// Supports multiple DEX protocols (UniswapV2, UniswapV3, Curve, etc.)
/// 
/// Core functionality:
/// - Simulates buy -> approve -> sell transactions sequentially with state preservation
/// - Dynamic sell amount based on actual tokens received
/// - Multi-protocol support through pool adapters
/// - Comprehensive tax calculation

pub mod config;
pub mod types;
pub mod analyzer;
pub mod pool_adapters;
pub mod tx_builders;
pub mod tax_calculator;
pub mod optional_setup_buy_approve_sell_token_simulator;

pub use config::PoolViabilityConfig;
pub use types::{PoolViabilityResult, PoolType};
pub use analyzer::analyze_pool_viability;
pub use optional_setup_buy_approve_sell_token_simulator::{
    OptionalSetupBuyApproveSellTokenSimulator, 
    OptionalSetupBuyApproveSellResult,
};