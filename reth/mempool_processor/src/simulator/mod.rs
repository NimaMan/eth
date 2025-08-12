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
pub mod sequential_tx_simulator;
pub mod single_tx_simulator;
pub mod unified_simulator;

pub use simulation_manager::{
    SimulationManager,
    SimulationRequest,
    SimulationType,
    SimulationResult,
    BuySellResult,
};
pub use simulation_queue::{SimulationQueue, QueueStats};
pub use sequential_tx_simulator::{
    SequentialBuySellSimulator,
    SequenceSimulationResult,
    TransactionSimulationResult,
    BuySellSimulatorConfig,
};
pub use single_tx_simulator::{
    TxSimulator,
    SimulationResult as TxSimulationResult,
    StateChangeResult,
    CallTraceResult,
    mempool_tx_to_call_request,
};
pub use unified_simulator::UnifiedSimulator;
