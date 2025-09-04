/// TX Simulator integration module for TxProcessor
/// 
/// Contains tx_simulator support functionality for the tx_processor core

pub mod unsigned_tx_builder;
pub mod erc20_token_buy_approve_sell_tx_simulator;

pub use unsigned_tx_builder::UnsignedTxBuilder;

// Re-export the main components for convenience
pub use erc20_token_buy_approve_sell_tx_simulator::{
    check_can_buy_sell_pool,
    PoolViabilityConfig,
    PoolViabilityResult,
    PoolType,
    OptionalSetupBuyApproveSellResult,
    TradingSequenceResult,
};