pub mod liquidity_removal_simulator;
pub mod mempool_simulator;
pub mod pool_buy_sell_simulator;
/// Simulation Manager Module - Per-Pool Transaction Simulation
///
/// CRITICAL ARCHITECTURE:
/// Simulations are performed PER-POOL, not per-token.
///
/// Key Design:
/// - Each pool of a token is simulated INDEPENDENTLY
/// - Buy/sell tests are run against SPECIFIC pools
/// - Results include pool_address and pool_type
/// - Currently supports V2 pools (V3/V4 filtered out)
///
/// Simulation Flow:
/// 1. Get all pools for a token from cache
/// 2. Filter to supported pool types (V2 only)
/// 3. FOR EACH POOL:
///    - Run transaction simulation
///    - Run buy simulation on THIS pool
///    - Run sell simulation on THIS pool
///    - Generate pool-specific result
/// 4. Return Vec<Result> with one entry per pool
pub mod simulation_manager;
pub mod simulation_queue;

pub use liquidity_removal_simulator::{LiquidityRemovalResult, LiquidityRemovalSimulator};
pub use mempool_simulator::{
    mempool_tx_to_unsigned_tx, AddressStateChange, MempoolSimulator,
    SimulationResult as TxSimulationResult, StateChangeResult,
};
pub use pool_buy_sell_simulator::{PoolBuySellSimulator, PoolSimulationResult};
pub use simulation_manager::{
    BuySellResult, SimulationManager, SimulationRequest, SimulationResult, SimulationType,
};
pub use simulation_queue::{QueueStats, SimulationQueue};
