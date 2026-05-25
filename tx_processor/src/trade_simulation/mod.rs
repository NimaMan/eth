pub mod buy_swap_simulator;
pub mod cross_venue_buy_approve_sell;
pub mod pool_buy_sell_simulator;
pub mod sell_swap;
pub mod types;
/// TX Simulator integration module for TxProcessor
///
/// Contains tx_simulator support functionality for the tx_processor core
// Re-export the main components for convenience
pub use pool_buy_sell_simulator::{
    check_can_buy_sell_pool, LivePoolBuySellSimulator, PoolBuySellSimulator,
};
pub use types::{
    OptionalSetupBuyApproveSellResult, PoolBuySellParameters, PoolBuySellSimulationResult,
    PoolType, TradingSequenceResult, UniswapV4PoolConfig,
};

// Re-export buy swap simulator
pub use buy_swap_simulator::{
    simulate_buy_swap, simulate_buy_swap_with_params, simulate_buy_swap_with_params_and_chain,
    BuySwapResult,
};
pub use cross_venue_buy_approve_sell::{
    simulate_cross_venue_buy_approve_sell, CrossVenueArbResult,
};
pub use sell_swap::{
    simulate_sell_swap, simulate_sell_swap_with_params, simulate_sell_swap_with_params_and_chain,
    SellSwapResult,
};
