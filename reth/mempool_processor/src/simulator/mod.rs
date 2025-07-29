/// Simulation Manager Module
/// 
/// Manages transaction simulations with priority queuing and result caching

pub mod simulation_manager;
pub mod simulation_queue;
pub mod buy_sell_sequence_simulator;
pub mod tx_simulator;

pub use simulation_manager::{
    SimulationManager,
    SimulationRequest,
    SimulationType,
    SimulationResult,
    BuySellResult,
    StateChange,
};
pub use simulation_queue::{SimulationQueue, QueueStats};
pub use buy_sell_sequence_simulator::{
    SequentialBuySellSimulator,
    SequenceSimulationResult,
    TransactionSimulationResult,
    BuySellSimulatorConfig,
};
pub use tx_simulator::{
    TxSimulator,
    SimulationResult as TxSimulationResult,
    StateChangeResult,
    CallTraceResult,
    mempool_tx_to_call_request,
};