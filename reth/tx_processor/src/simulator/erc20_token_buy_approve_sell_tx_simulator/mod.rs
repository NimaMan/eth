/// Trading Viability Analysis Module
/// 
/// Checks if a token can be bought and sold on a DEX pool by simulating the complete
/// trading sequence and calculating taxes/fees.
/// 
/// Supports multiple DEX protocols (UniswapV2, UniswapV3, Curve, etc.)
/// 
/// Core functionality:
/// - Simulates buy -> approve -> sell transactions sequentially with state preservation
/// - Dynamic sell amount based on actual tokens received
/// - Multi-protocol support through pool adapters
/// - Comprehensive tax calculation

pub mod config;
pub mod types;
pub mod pool_buy_sell_simulator;
pub mod pool_adapters;
pub mod tx_builders;
pub mod tax_calculator;
pub mod simulation_result;

pub use config::PoolViabilityConfig;
pub use types::{PoolViabilityResult, PoolType};
pub use pool_buy_sell_simulator::check_can_buy_sell_pool;
pub use simulation_result::{TradingSequenceResult, OptionalSetupBuyApproveSellResult};