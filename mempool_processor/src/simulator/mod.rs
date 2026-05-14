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
/// - V2/Sushi and V3 pools are probed when the token cache has enough metadata
/// - V4 removal intent is detected from processed events; V4 buy/sell probes
///   stay gated until the cache exposes full pool-key simulation config
///
/// Simulation Flow:
/// 1. Get all pools for a token from cache
/// 2. Resolve protocol-specific simulation config for each supported pool
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
    is_funding_dependency_error, is_pending_nonce_dependency_error, BuySellResult,
    SimulationManager, SimulationResult, SimulationType, TxSimulationJob,
};
pub use simulation_queue::{QueueStats, SimulationQueue};
