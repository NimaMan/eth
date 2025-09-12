/// TX Simulator integration module for TxProcessor
/// 
/// Contains tx_simulator support functionality for the tx_processor core

pub mod unsigned_tx_builder;
pub mod signed_tx_builder;
pub mod buy_swap_simulator;
pub mod sell_swap_simulator;
pub mod config;
pub mod types;
pub mod pool_buy_sell_simulator;
pub mod cross_venue_buy_approve_sell;

pub use unsigned_tx_builder::UnsignedTxBuilder;
pub use signed_tx_builder::SignedTxBuilder;

// Re-export the main components for convenience
pub use config::PoolViabilityConfig;
pub use types::{
    PoolViabilityResult,
    PoolType,
    OptionalSetupBuyApproveSellResult,
    TradingSequenceResult,
};
pub use pool_buy_sell_simulator::check_can_buy_sell_pool;

// Re-export buy swap simulator
pub use buy_swap_simulator::{simulate_buy_swap, BuySwapResult};
pub use sell_swap_simulator::{simulate_sell_swap, SellSwapResult};
pub use cross_venue_buy_approve_sell::{simulate_cross_venue_buy_approve_sell, CrossVenueArbResult};
