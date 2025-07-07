/// Transaction Simulator Module - Direct Reth Integration Only
/// 
/// This module provides ultra-fast transaction simulation by directly
/// accessing Reth's database, bypassing RPC entirely for 20-40x speedup.
/// 
/// All simulation is now done through reth_tx_simulator which provides:
/// - Direct database access (no RPC overhead)
/// - Sub-millisecond simulation times
/// - Full state change extraction with ETH/token transfers
/// - 100% compatibility with debug_traceCall output
/// 
/// Usage:
/// ```rust
/// use mempool_processor::tx_simulator::DirectTxSimulator;
/// 
/// let simulator = DirectTxSimulator::new("/path/to/reth/db")?;
/// let result = simulator.simulate_mempool_tx(&tx).await?;
/// ```

pub mod direct_simulator;

// Re-export key types
pub use direct_simulator::{
    DirectTxSimulator,
    SimulationResult,
    StateChangeResult,
    CallTraceResult,
    mempool_tx_to_call_request,
};

// Re-export reth_tx_simulator types for convenience
pub use reth_tx_simulator::{
    RethDirectTxSimulator,
    CallRequest,
    TransactionStateChanges,
    AddressStateChange,
    DebugAddressStateChange,
    ipc_to_call_request,
};